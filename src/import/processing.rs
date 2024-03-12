use std::collections::HashMap;
use std::io::{BufRead, Read};
use std::iter;
use std::sync::{Arc, RwLock};
use bincode::de::read::Reader;
use hayagriva::{io, Library};
use hayagriva::io::BibLaTeXError;
use html_parser::{Dom, Node};
use pandoc::{InputFormat, InputKind, OutputFormat, OutputKind, PandocError, PandocOutput};
use rocket::fs::TempFile;
use rocket::http::ContentType;
use crate::data_storage::{BibEntry, ProjectData, ProjectStorage};
use crate::settings::Settings;
use tokio::io::AsyncReadExt;
use crate::projects::{BlockData, BlockType, NewContentBlock, Section, SectionMetadata, SectionOrToc};
use crate::utils::block_id_generator::generate_id;

pub struct ImportProcessor{
    pub settings: Settings,
    pub project_storage: Arc<ProjectStorage>,
    pub job_queue: RwLock<Vec<ImportJob>>,
    pub job_archive: RwLock<Vec<Arc<RwLock<ImportJob>>>>,
}

pub enum ImportStatus{
    Pending,
    Processing,
    Complete,
    Failed
}

#[derive(Debug)]
pub enum ImportError{
    UnknownFileType,
    UnsupportedFileType,
    InvalidFile,
    BibFileInvalid,
    PandocError,
    HtmlConversionFailed
}

pub struct ImportJob{
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub length: usize,
    pub processed: usize,
    pub files_to_process: Vec<(String, ContentType)>,
    pub bib_file: Option<String>,
    pub status: ImportStatus,
}

impl ImportProcessor{
    pub fn start(settings: Settings, project_storage: Arc<ProjectStorage>) -> Arc<ImportProcessor>{
        let processor = Arc::new(ImportProcessor{
            settings,
            project_storage,
            job_queue: RwLock::new(Vec::new()),
            job_archive: RwLock::new(Vec::new()),
        });

        let processor_clone = processor.clone();
        tokio::spawn(async move {
            let running_threads: Arc<std::sync::atomic::AtomicU64> = Arc::new(std::sync::atomic::AtomicU64::new(0));

            loop{
                // Check if there are any new jobs
                let job_queue_len = processor_clone.job_queue.read().unwrap().len();
                if job_queue_len > 0 && processor_clone.settings.max_import_threads > running_threads.load(std::sync::atomic::Ordering::Relaxed){
                    println!("Starting new import job..."); //TODO: new thread

                    let proc_clone = processor_clone.clone();
                    let running_threads_cpy = running_threads.clone();

                    tokio::spawn(async move{
                        running_threads_cpy.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let mut job = match proc_clone.job_queue.write().unwrap().pop(){
                            Some(job) => job,
                            None => {
                                running_threads_cpy.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                                return;
                            }
                        };
                        job.status = ImportStatus::Processing;
                        let job = Arc::new(RwLock::new(job));
                        proc_clone.job_archive.write().unwrap().push(job.clone());
                        proc_clone.process_job(job, proc_clone.project_storage.clone()).await;
                        println!("Job finished");
                        running_threads_cpy.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                    });
                }else{
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        });

        processor
    }

    async fn process_job(&self, mut job: Arc<RwLock<ImportJob>>, project_storage: Arc<ProjectStorage>){
        let job = job.clone();

        let project_id = job.read().unwrap().project_id.clone();
        let bib_file = job.read().unwrap().bib_file.clone();

        // Start with bib file if present
        if let Some(bib_file) = bib_file{
            match self.import_bib_entries(project_id, bib_file, &self.settings).await{
                Ok(_) => {
                    println!("Bib entries imported successfully");
                }
                Err(e) => {
                    println!("Error importing bib entries: {:?}", e);
                    job.write().unwrap().status = ImportStatus::Failed;
                    return;
                }
            }
        }

        let files_to_process = job.read().unwrap().files_to_process.clone();

        loop{
            println!("Checking remaining files... {} remaining", job.read().unwrap().files_to_process.len());
            let res = job.write().unwrap().files_to_process.pop();
            let (file, content_type) = match res{
                Some(f) => f,
                None => {
                    job.write().unwrap().status = ImportStatus::Complete;
                    break;
                }
            };

            let project_id = job.read().unwrap().project_id.clone();

            let project = project_storage.get_project(&project_id, &self.settings).await.unwrap();

            match self.convert_file(&file, content_type, project).await{
                Ok(_) => {
                    println!("File processed successfully");
                    // Remove file from temp directory
                    let res = tokio::fs::remove_file(file).await;
                    if let Err(e) = res{
                        println!("Error removing file from temp directory: {:?}", e);
                    }
                }
                Err(e) => {
                    println!("Error processing file: {:?}", e);
                    job.write().unwrap().status = ImportStatus::Failed;
                    // Remove files from temp directory
                    let res = tokio::fs::remove_file(file).await;
                    if let Err(e) = res{
                        println!("Error removing file from temp directory: {:?}", e);
                    }
                    for (file, _) in files_to_process.iter(){
                        let res = tokio::fs::remove_file(file).await;
                        if let Err(e) = res{
                            println!("Error removing file from temp directory: {:?}", e);
                        }
                    }
                    break;
                }
            }

            job.write().unwrap().processed += 1;
        }
    }

    async fn convert_file(&self, file_path: &str, content_type: ContentType, project: Arc<RwLock<ProjectData>>) -> Result<(), ImportError>{
        let mut file = match tokio::fs::File::open(file_path).await{
            Ok(file) => file,
            Err(e) => {
                eprintln!("Couldn't open file to import: {}", e);
                return Err(ImportError::InvalidFile)
            }
        };

        let mut file_content = String::new();
        let mut marks: Vec<String> = vec![];
        if let Err(e) = file.read_to_string(&mut file_content).await{
            eprintln!("Error reading file to import: {}", e);
            return Err(ImportError::InvalidFile);
        };

        match content_type.to_string().as_str() {
                    "text/x-tex" | "application/x-tex" => {
                        println!("Processing LaTeX file");
                        (file_content, marks) = preprocess::latex(file_content);
                        file_content = self.convert_with_pandoc(file_content, InputFormat::Latex)?;
                        file_content = postprocess::latex(file_content, marks);
                    },
                    "application/vnd.oasis.opendocument.text" => {
                        println!("Processing ODT file");
                        file_content = self.convert_with_pandoc(file_content, InputFormat::Other("ODT".to_string()))?;
                    },
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                        println!("Processing DOCX file");
                        file_content = self.convert_with_pandoc(file_content, InputFormat::Docx)?;
                    },
                    "application/msword" => {
                        println!("Processing DOC file");
                        file_content = self.convert_with_pandoc(file_content, InputFormat::Other("DOC".to_string()))?;
                    },
                    "application/epub+zip" => {
                        println!("Processing EPUB file");
                        file_content = self.convert_with_pandoc(file_content, InputFormat::Epub)?;
                    },
                    "application/rtf" => {
                        println!("Processing RTF file");
                        file_content = self.convert_with_pandoc(file_content, InputFormat::Rtf)?;
                    },
                    "text/markdown" | "text/x-markdown" => {
                        println!("Processing Markdown file");
                        file_content = self.convert_with_pandoc(file_content, InputFormat::Markdown)?;
                    },
                    _ => {
                        println!("Unsupported file type: {}", content_type);
                        return Err(ImportError::UnsupportedFileType);
                    }
        }

        self.import_html(file_content, project)?;
        Ok(())
    }

    fn convert_with_pandoc(&self, input: String, input_format: InputFormat) -> Result<String, ImportError>{
           let mut pandoc = pandoc::new();
            pandoc.set_input(InputKind::Pipe(input));
            pandoc.set_input_format(input_format, vec![]);
            pandoc.set_output_format(OutputFormat::Html5, vec![]);
            pandoc.set_output(OutputKind::Pipe);
            match pandoc.execute(){
                Ok(res) => {
                    match res{
                        PandocOutput::ToFile(_) => Err(ImportError::PandocError),
                        PandocOutput::ToBuffer(res) => Ok(res),
                        PandocOutput::ToBufferRaw(_) => Err(ImportError::PandocError)
                    }
                },
                Err(e) => {
                    println!("Couldn't convert import file with pandoc: {}", e);
                    Err(ImportError::PandocError)
                }
            }
    }

    fn import_html(&self, input: String, project_data: Arc<RwLock<ProjectData>>) -> Result<(), ImportError>{
        let dom = match Dom::parse(&input){
            Ok(dom) => dom,
            Err(e) => {
                eprintln!("Couldn't parse html from import after pandoc: {}", e);
                return Err(ImportError::HtmlConversionFailed)
            }
        };
        if dom.tree_type == html_parser::DomVariant::Document{
            return Err(ImportError::HtmlConversionFailed)
        } //TODO support a full html document
        
        let mut section = Section{
            id: Some(uuid::Uuid::new_v4()),
            css_classes: vec![],
            sub_sections: vec![],
            children: vec![],
            visible_in_toc: true,
            metadata: SectionMetadata {
                title: "Imported Section".to_string(),
                subtitle: None,
                authors: vec![],
                editors: vec![],
                web_url: None,
                identifiers: vec![],
                published: None,
                last_changed: None,
                lang: None,
            },
        };

        // Get footnotes:
        let mut footnotes : HashMap<String, String> = HashMap::new();

        if let Some(aside) = dom.children.iter().find(|x| {
            match x{
                Node::Element(el) => el.name == "aside",
                _ => false
            }
        }){
            if let Node::Element(aside) = aside{
                if aside.id == Some("footnotes".to_string()){
                    let ol = aside.children.iter().find(|node| match node{
                        Node::Element(el) => el.name == "ol",
                        _ => false
                    });
                    if let Some(ol) = ol{
                        if let Node::Element(ol) = ol{
                            for node in ol.children.iter(){
                                if let Node::Element(li) = node{
                                    if let Some(id) = li.id.clone(){
                                        let id = id.to_string();
                                        let mut text = String::new();
                                        if let Some(ptag) = li.children.iter().next(){
                                            match ptag {
                                                Node::Element(ptag) => {
                                                    for node in ptag.children.iter() {
                                                        match node {
                                                            Node::Text(t) => text.push_str(&t),
                                                            Node::Element(ele) => {
                                                                match ele.name.to_lowercase().as_str() {
                                                                    "a" => {
                                                                        if let Some(role) = ele.attributes.get("role") {
                                                                            if let Some(role) = role {
                                                                                if role == "doc-backlink" {
                                                                                    // Skip backlinks
                                                                                    continue;
                                                                                }
                                                                            }
                                                                        }
                                                                        let mut attributes = String::new();
                                                                        for (attr, attrvalue) in ele.attributes.iter(){
                                                                            match attrvalue{
                                                                                Some(value) => attributes.push_str(&format!(" {}=\"{}\"", attr, value)),
                                                                                None => attributes.push_str(&format!(" {}", attr)),
                                                                            }
                                                                        }
                                                                        text.push_str(&format!("<a {}>{}</a>", attributes, &self.dom_to_html(ele.clone(), None)));
                                                                    },
                                                                    _ => {
                                                                        // TODO: whitelist elements and attributes
                                                                        // Currently we allow all elements but strip attributes
                                                                        text.push_str(&format!("<{}>{}</{}>", ele.name, &self.dom_to_html(ele.clone(), None), ele.name));
                                                                    },
                                                                }
                                                            }
                                                            _ => {}
                                                        }

                                                    }
                                                },
                                                Node::Text(t) => {
                                                    text.push_str(&t);
                                                },
                                                _ => {}
                                            }
                                        }
                                        footnotes.insert(id.clone(), text.clone());
                                        println!("Found footnote: {}:{}", id, text);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for node in dom.children{
            match node{
                Node::Text(t) => {
                    // Wrap text without a tag in a paragraph
                    let cb = NewContentBlock{
                        id: generate_id(&section),
                        block_type: BlockType::Paragraph,
                        data: BlockData::Paragraph {
                            text: t,
                        },
                        revision_id: None,
                    };
                    section.children.push(cb);
                }
                Node::Element(el) => {
                    match el.name.to_lowercase().as_str(){
                        "h1" | "h2" | "h4" | "h5" | "h6" => {
                            let level = match el.name.to_lowercase().as_str(){
                                "h1" => 1,
                                "h2" => 2,
                                "h3" => 3,
                                "h4" => 4,
                                "h5" => 5,
                                "h6" => 6,
                                _ => 0
                            };
                            section.children.push(NewContentBlock{
                                id: generate_id(&section),
                                block_type: BlockType::Heading,
                                data: BlockData::Heading {
                                    text: self.dom_to_html(el, Some(&footnotes)),
                                    level,
                                },
                                revision_id: None,
                            })
                        },
                        "p" => {
                            section.children.push(NewContentBlock{
                                id: generate_id(&section),
                                block_type: BlockType::Paragraph,
                                data: BlockData::Paragraph {
                                    text: self.dom_to_html(el, Some(&footnotes)),
                                },
                                revision_id: None,
                            })
                        },
                        "ul" | "ol" => {
                            let style = match el.name.to_lowercase().as_str(){
                                "ul" => "unordered",
                                "ol" => "ordered",
                                _ => "unordered"
                            };
                            let style = style.to_string();
                            let items = el.children.iter().filter_map(|node| match node{
                                Node::Element(el) => {
                                    if el.name.to_lowercase() == "li"{
                                        Some(self.dom_to_html(el.clone(), Some(&footnotes)))
                                    }else{
                                        None
                                    }
                                },
                                _ => None
                            }).collect();

                            section.children.push(NewContentBlock{
                                id: generate_id(&section),
                                block_type: BlockType::List,
                                data: BlockData::List {
                                    style,
                                    items
                                },
                                revision_id: None,
                            });
                        },
                        "blockquote" => {
                            section.children.push(NewContentBlock{
                                id: generate_id(&section),
                                block_type: BlockType::Quote,
                                data: BlockData::Quote {
                                    text: self.dom_to_html(el, Some(&footnotes)),
                                    caption: "".to_string(),
                                    alignment: "".to_string(),
                                },
                                revision_id: None,
                            });
                        },
                        "aside" => {
                            if let Some(id) = el.id{
                                if id == "footnotes"{
                                    // Skip footnotes
                                    continue;
                                }
                            }
                        },
                        _ => {
                            println!("Warning: Unsupported tag: {}", el.name);
                            // Add as paragraph
                            section.children.push(NewContentBlock{
                                id: generate_id(&section),
                                block_type: BlockType::Paragraph,
                                data: BlockData::Paragraph {
                                    text: self.dom_to_html(el, Some(&footnotes)),
                                },
                                revision_id: None,
                            });
                        }
                    }
                }
                // Skip comments
                Node::Comment(_) => {}
            }
        }

        project_data.write().unwrap().sections.push(SectionOrToc::Section(section));
        Ok(())
    }

    //TODO: maybe also copy classes and ids from the html
    fn dom_to_html(&self, ele: html_parser::Element, footnotes: Option<&HashMap<String, String>>) -> String{
        let mut html = String::new();
        for node in ele.children{
            match node{
                Node::Text(t) => {
                    println!("Found Text: {}", t);
                    html.push_str(&t);
                }
                Node::Element(el) => {
                    println!("Found Element: {}", el.name);

                    if el.name == "a"{
                        if let Some(role) = el.attributes.get("role"){
                            if let Some(role) = role {
                                if role == "doc-noteref" {
                                    // This is a reference to a footnote
                                    if let Some(sup) = el.children.iter().next() {
                                        if let Node::Element(sup) = sup {
                                            if sup.name == "sup" {
                                                if let Some(num) = sup.children.iter().next() {
                                                    if let Node::Text(num) = num {
                                                        if let Some(footnotes) = footnotes{
                                                            let num = num.trim().to_string();
                                                            if let Some(footnote) = footnotes.get(&format!("fn{}", num)){
                                                                println!("Found footnote: {}", footnote.clone());
                                                                html.push_str(&format!("<span class=\"note\" note-type=\"footnote\" note-content=\"{}\">F</span>", footnote.clone().replace("\"", "'")));
                                                                continue;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let mut attrs : String = String::new();
                    for (attr, attrvalue) in el.attributes.iter(){
                        match attrvalue{ //TODO: whitelist attributes that are allowed for each tag, e.g. href for a-tags
                            Some(value) => attrs.push_str(&format!(" {}=\"{}\"", attr, value)),
                            None => attrs.push_str(&format!(" {}", attr)),
                        }
                    }
                    html.push_str(&format!("<{}{}>", el.name, attrs));
                    html.push_str(&self.dom_to_html(el.clone(), footnotes));
                    html.push_str(&format!("</{}>", el.name));
                },
                // Ignore comments
                Node::Comment(_) => {}
            }
        }
        println!("Debug: {}", html);
        html
    }

    async fn import_bib_entries(&self, project_id: uuid::Uuid, bib_file_path: String, settings: &Settings) -> Result<(), ImportError>{
        let mut bib_file_content = String::new();
        let mut bib_file = match tokio::fs::File::open(bib_file_path.clone()).await{
            Ok(bib_file) => bib_file,
            Err(e) => {
                println!("Error opening bib file {}: {}", bib_file_path, e);
                return Err(ImportError::BibFileInvalid);
            }
        };
        if let Err(e) = bib_file.read_to_string(&mut bib_file_content).await{
            println!("Error reading bib file: {}", e);
            return Err(ImportError::BibFileInvalid);
        }
        let bib = match io::from_biblatex_str(&bib_file_content){
            Ok(bib) => bib,
            Err(e) => {
                println!("Error parsing bib file: {}", e.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", "));
                return Err(ImportError::BibFileInvalid);
            }
        };

        let mut project_storage = self.project_storage.clone();
        let mut project = project_storage.get_project(&project_id, settings).await.unwrap().clone();
        for entry in bib.iter(){
            let converted = BibEntry::from(entry);
            project.write().unwrap().bibliography.insert(converted.key.clone(), converted);
        }

        Ok(())
    }
}

/// Contains preprocessing methods that get called, BEFORE pandoc is executed.
mod preprocess{
    use regex::Regex;

    /// Preprocessing for latex input
    /// Replaces all endnotes with footnotes since endnotes are not supported by pandoc
    /// Finds all citations and replaces them with a temporary mark which survives pandoc
    pub fn latex(mut input: String) -> (String, Vec<String>){
        let mut marks = Vec::new();

        let re = Regex::new(r"\\(cite|footcite|footcitetext|fullcite|footfullcite)(?:\[[^\]]*?\])?(?:\[[^\]]*?\])?\{(.*?)\}").unwrap();
        input = re.replace_all(&input, |caps: &regex::Captures|{
                let key = &caps[2];
                marks.push(key.to_string());
                return format!("vb-cite-{}", marks.len()-1)
            }
        ).to_string();

        (input.replace("\\endnote", "\\footnote"), marks)
    }
}

mod postprocess{
    use regex::Regex;

    pub fn latex(mut input: String, marks: Vec<String>) -> String{
        let re = Regex::new(r"vb-cite-(\d+)").unwrap();

        // Replace temporary citation marks with actual citations
        input = re.replace_all(&input, |caps: &regex::Captures| {
            let num = match (&caps[1]).parse::<usize>() {
                Ok(num) => num,
                Err(e) => {
                    println!("Warning: couldn't parse vb-cite- citation number: {}", e);
                    return String::from("invalid-citation!");
                }
            };
            format!("<citation data-key=\"{}\">C</citation>", marks.get(num).unwrap_or(&"".to_string()))
        }).to_string();

        input
    }
}