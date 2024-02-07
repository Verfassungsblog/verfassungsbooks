use std::time::SystemTime;
use crate::data_storage::DataStorage;
use crate::projects::{InnerContentBlock, PatchInnerContentBlock, SectionMetadata};
use crate::projects::SectionOrToc;
use rocket::serde::json::Json;
use std::sync::Arc;
use bincode::{Decode, Encode};
use chrono::NaiveDateTime;
use rocket::State;
use serde_derive::{Deserialize, Serialize};
use crate::data_storage::ProjectStorage;
use crate::projects::{Identifier, Keyword, ContentBlock, Language, License, ProjectMetadata, ProjectSettings, Section};
use crate::session::session_guard::Session;
use crate::settings::Settings;

/// Api Endpoints for the project editor

#[derive(Serialize, Deserialize)]
pub struct ApiResult<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

#[derive(Serialize, Deserialize)]
pub enum ApiError{
    NotFound,
    BadRequest(String),
    Unauthorized,
    Other(String),
}

impl<T> ApiResult<T>{
    pub fn new_error(error: ApiError) -> Json<ApiResult<T>> {
        Json(Self {
            error: Some(error),
            data: None,
        })
    }
    pub fn new_data(data: T) -> Json<ApiResult<T>> {
        Json(Self {
            error: None,
            data: Some(data),
        })
    }
}

#[get("/api/projects/<project_id>/metadata")]
pub async fn get_project_metadata(project_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<Option<ProjectMetadata>>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let metadata = project_entry.read().unwrap().metadata.clone();

    // TODO: Check if all authors and editors still exist, if not, remove them from the metadata and save the project

    ApiResult::new_data(metadata)

}
pub trait Patch<P, T>{
    fn patch(&mut self, patch: P) -> T;
}

impl Patch<PatchProjectMetadata, ProjectMetadata> for ProjectMetadata{
    fn patch(&mut self, patch: PatchProjectMetadata) -> ProjectMetadata{
        let mut new_metadata = self.clone();

        if let Some(title) = patch.title{
            new_metadata.title = title;
        }

        if let Some(subtitle) = patch.subtitle{
            new_metadata.subtitle = subtitle;
        }

        if let Some(authors) = patch.authors{
            new_metadata.authors = authors;
        }

        if let Some(editors) = patch.editors{
            new_metadata.editors = editors;
        }

        if let Some(web_url) = patch.web_url{
            new_metadata.web_url = web_url;
        }

        if let Some(identifiers) = patch.identifiers{
            new_metadata.identifiers = identifiers;
        }

        if let Some(published) = patch.published{
            new_metadata.published = published;
        }

        if let Some(languages) = patch.languages{
            new_metadata.languages = languages;
        }

        if let Some(number_of_pages) = patch.number_of_pages{
            new_metadata.number_of_pages = number_of_pages;
        }

        if let Some(short_abstract) = patch.short_abstract{
            new_metadata.short_abstract = short_abstract;
        }

        if let Some(long_abstract) = patch.long_abstract{
            new_metadata.long_abstract = long_abstract;
        }

        if let Some(keywords) = patch.keywords{
            new_metadata.keywords = keywords;
        }

        if let Some(ddc) = patch.ddc{
            new_metadata.ddc = ddc;
        }

        if let Some(license) = patch.license{
            new_metadata.license = license;
        }

        if let Some(series) = patch.series{
            new_metadata.series = series;
        }

        if let Some(volume) = patch.volume{
            new_metadata.volume = volume;
        }

        if let Some(edition) = patch.edition{
            new_metadata.edition = edition;
        }

        if let Some(publisher) = patch.publisher{
            new_metadata.publisher = publisher;
        }

        new_metadata
    }
}

/// Struct for patching a section
/// Does NOT allow to patch the content of a section, use the content_block endpoints or move endpoints for that
#[derive(Deserialize, Serialize, Debug, Encode, Decode, Clone, PartialEq, Default)]
pub struct PatchSection{
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    #[bincode(with_serde)]
    pub id: Option<Option<uuid::Uuid>>,
    pub css_classes: Option<Vec<String>>,
    pub visible_in_toc: Option<bool>,
    pub metadata: Option<PatchSectionMetadata>
}

#[derive(Deserialize, Serialize, Debug, Encode, Decode, Clone, PartialEq, Default)]
pub struct PatchSectionMetadata {
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub subtitle: Option<Option<String>>,
    #[bincode(with_serde)]
    pub authors: Option<Vec<uuid::Uuid>>,
    #[bincode(with_serde)]
    pub editors: Option<Vec<uuid::Uuid>>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub web_url: Option<Option<String>>,
    pub identifiers: Option<Vec<Identifier>>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    #[bincode(with_serde)]
    pub published: Option<Option<NaiveDateTime>>,
    #[bincode(with_serde)]
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub last_changed: Option<Option<NaiveDateTime>>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub lang: Option<Option<Language>>,
}

impl Patch<PatchSectionMetadata, SectionMetadata> for SectionMetadata{
    fn patch(&mut self, patch: PatchSectionMetadata) -> SectionMetadata{
        let mut new_metadata = self.clone();

        if let Some(title) = patch.title{
            new_metadata.title = title;
        }

        if let Some(subtitle) = patch.subtitle{
            new_metadata.subtitle = subtitle;
        }

        if let Some(authors) = patch.authors{
            new_metadata.authors = authors;
        }

        if let Some(editors) = patch.editors{
            new_metadata.editors = editors;
        }

        if let Some(web_url) = patch.web_url{
            new_metadata.web_url = web_url;
        }

        if let Some(identifiers) = patch.identifiers{
            new_metadata.identifiers = identifiers;
        }

        if let Some(published) = patch.published{
            new_metadata.published = published;
        }

        if let Some(last_changed) = patch.last_changed{
            new_metadata.last_changed = last_changed;
        }

        if let Some(lang) = patch.lang{
            new_metadata.lang = lang;
        }

        new_metadata
    }
}

// Implement patch for PatchSection
impl Patch<PatchSection, Section> for Section{
    fn patch(&mut self, patch: PatchSection) -> Section{
        let mut new_section = self.clone();

        if let Some(id) = patch.id{
            new_section.id = id;
        }

        if let Some(css_classes) = patch.css_classes{
            new_section.css_classes = css_classes;
        }

        if let Some(visible_in_toc) = patch.visible_in_toc{
            new_section.visible_in_toc = visible_in_toc;
        }

        if let Some(metadata) = patch.metadata{
            new_section.metadata = self.metadata.patch(metadata);
        }

        new_section
    }
}

#[derive(Deserialize, Serialize, Debug, Encode, Decode, Clone, PartialEq, Default)]
pub struct PatchProjectMetadata{
    /// Book Title
    pub title: Option<String>,
    /// Subtitle of the book
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub subtitle: Option<Option<String>>,
    /// List of ids of authors of the book
    #[bincode(with_serde)]
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub authors: Option<Option<Vec<uuid::Uuid>>>,
    /// List of ids of editors of the book
    #[bincode(with_serde)]
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub editors: Option<Option<Vec<uuid::Uuid>>>,
    /// URL to a web version of the book or reference
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub web_url: Option<Option<String>>,
    /// List of identifiers of the book (e.g. ISBNs)
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub identifiers: Option<Option<Vec<Identifier>>>,
    /// Date of publication
    #[bincode(with_serde)]
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub published: Option<Option<NaiveDateTime>>,
    /// Languages of the book
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub languages: Option<Option<Vec<Language>>>,
    /// Number of pages of the book (should be automatically calculated)
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub number_of_pages: Option<Option<u32>>,
    /// Short abstract of the book
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub short_abstract: Option<Option<String>>,
    /// Long abstract of the book
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub long_abstract: Option<Option<String>>,
    /// Keywords of the book
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub keywords: Option<Option<Vec<Keyword>>>,
    /// Dewey Decimal Classification (DDC) classes (subject groups)
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub ddc: Option<Option<String>>,
    /// License of the book
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub license: Option<Option<License>>,
    /// Series the book belongs to
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub series: Option<Option<String>>,
    /// Volume of the book in the series
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub volume: Option<Option<String>>,
    /// Edition of the book
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub edition: Option<Option<String>>,
    /// Publisher of the book
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub publisher: Option<Option<String>>
}

#[post("/api/projects/<project_id>/metadata", data = "<metadata>")]
pub async fn set_project_metadata(project_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>, metadata: Json<ProjectMetadata>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();

    project.metadata = Some(metadata.into_inner());

    ApiResult::new_data(())
}

#[patch("/api/projects/<project_id>/metadata", data = "<metadata>")]
pub async fn patch_project_metadata(project_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>, data_storage: &State<Arc<DataStorage>>, metadata: Json<PatchProjectMetadata>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };


    let mut old_metadata = match &project_entry.read().unwrap().metadata{
        Some(metadata) => metadata.clone(),
        None => {
            ProjectMetadata::default()
        }
    };

    let new_metadata = old_metadata.patch(metadata.into_inner());

    // Validate new metadata

    // Validate authors: Check if each author exists
    if let Some(ref authors) = new_metadata.authors {
        for author in authors.iter() {
            if !data_storage.person_exists(author){
                return ApiResult::new_error(ApiError::BadRequest(format!("Author {} does not exist", author)));
            }
        }
    }
    // Validate editors: Check if each editor exists
    if let Some(ref editors) = new_metadata.editors {
        for editor in editors.iter() {
            if !data_storage.person_exists(editor){
                return ApiResult::new_error(ApiError::BadRequest(format!("Editor {} does not exist", editor)));
            }
        }
    }

    let mut project = project_entry.write().unwrap();

    project.metadata = Some(new_metadata);

    ApiResult::new_data(())
}

#[get("/api/projects/<project_id>/settings")]
pub async fn get_project_settings(project_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<Option<ProjectSettings>>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let settings = project_entry.read().unwrap().settings.clone();

    ApiResult::new_data(settings)
}

#[post("/api/projects/<project_id>/settings", data = "<project_settings>")]
pub async fn set_project_settings(project_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>, project_settings: Json<ProjectSettings>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();

    project.settings = Some(project_settings.into_inner());

    ApiResult::new_data(())
}

/// PUT /api/projects/<project_id>/metadata/authors/<author_id>
/// Add person as author to project
#[put("/api/projects/<project_id>/metadata/authors/<author_id>")]
pub async fn add_author_to_project(project_id: String, author_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    let author_id = match uuid::Uuid::parse_str(&author_id) {
        Ok(author_id) => author_id,
        Err(e) => {
            eprintln!("Couldn't parse author id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse author id".to_string()));
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();

    if let None = project.metadata{
        let new_metadata: ProjectMetadata = Default::default();
        project.metadata = Some(new_metadata);
    }

    if let None = project.metadata.as_ref().unwrap().authors{
        project.metadata.as_mut().unwrap().authors = Some(Vec::new());
    }

    if !project.metadata.as_ref().unwrap().authors.as_ref().unwrap().contains(&author_id){
        project.metadata.as_mut().unwrap().authors.as_mut().unwrap().push(author_id);
    }

    ApiResult::new_data(())
}

/// PUT /api/projects/<project_id>/metadata/editors/<editor_id>
/// Add person as editor to project
#[put("/api/projects/<project_id>/metadata/editors/<editor_id>")]
pub async fn add_editor_to_project(project_id: String, editor_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    let editor_id = match uuid::Uuid::parse_str(&editor_id) {
        Ok(editor_id) => editor_id,
        Err(e) => {
            eprintln!("Couldn't parse editor id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse editor id".to_string()));
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();

    if let None = project.metadata{
        let new_metadata: ProjectMetadata = Default::default();
        project.metadata = Some(new_metadata);
    }

    if let None = project.metadata.as_ref().unwrap().editors{
        project.metadata.as_mut().unwrap().editors = Some(Vec::new());
    }

    if !project.metadata.as_ref().unwrap().editors.as_ref().unwrap().contains(&editor_id){
        project.metadata.as_mut().unwrap().editors.as_mut().unwrap().push(editor_id);
    }

    ApiResult::new_data(())
}

/// DELETE /api/projects/<project_id>/metadata/authors/<author_id>
/// Remove person from project as author
#[delete("/api/projects/<project_id>/metadata/authors/<author_id>")]
pub async fn remove_author_from_project(project_id: String, author_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    let author_id = match uuid::Uuid::parse_str(&author_id) {
        Ok(author_id) => author_id,
        Err(e) => {
            eprintln!("Couldn't parse author id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse author id".to_string()));
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();

    if let None = project.metadata{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let None = project.metadata.as_ref().unwrap().authors{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let Some(index) = project.metadata.as_ref().unwrap().authors.as_ref().unwrap().iter().position(|x| *x == author_id){
        project.metadata.as_mut().unwrap().authors.as_mut().unwrap().remove(index);
    }

    ApiResult::new_data(())
}

/// DELETE /api/projects/<project_id>/metadata/editors/<editor_id>
/// Remove person from project as editor
#[delete("/api/projects/<project_id>/metadata/editors/<editor_id>")]
pub async fn remove_editor_from_project(project_id: String, editor_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    let editor_id = match uuid::Uuid::parse_str(&editor_id) {
        Ok(editor_id) => editor_id,
        Err(e) => {
            eprintln!("Couldn't parse author id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse editor id".to_string()));
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();

    if let None = project.metadata{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let None = project.metadata.as_ref().unwrap().editors{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let Some(index) = project.metadata.as_ref().unwrap().editors.as_ref().unwrap().iter().position(|x| *x == editor_id){
        project.metadata.as_mut().unwrap().editors.as_mut().unwrap().remove(index);
    }else{
        return ApiResult::new_error(ApiError::NotFound);
    }

    ApiResult::new_data(())
}

/// PUT /api/projects/<project_id>/metadata/keywords
/// Add keyword to project
#[put("/api/projects/<project_id>/metadata/keywords", data = "<keyword>")]
pub async fn add_keyword_to_project(project_id: String, keyword: Json<Keyword>, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();

    if let None = project.metadata{
        let new_metadata: ProjectMetadata = Default::default();
        project.metadata = Some(new_metadata);
    }

    if let None = project.metadata.as_ref().unwrap().keywords{
        project.metadata.as_mut().unwrap().keywords = Some(Vec::new());
    }

    if !project.metadata.as_ref().unwrap().keywords.as_ref().unwrap().contains(&keyword){
        project.metadata.as_mut().unwrap().keywords.as_mut().unwrap().push(keyword.into_inner());
    }

    ApiResult::new_data(())
}

/// DELETE /api/projects/<project_id>/metadata/keywords/<keyword>
/// Remove keyword from project
#[delete("/api/projects/<project_id>/metadata/keywords/<keyword>")]
pub async fn remove_keyword_from_project(project_id: String, keyword: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();

    if let None = project.metadata{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let None = project.metadata.as_ref().unwrap().keywords{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let Some(index) = project.metadata.as_ref().unwrap().keywords.as_ref().unwrap().iter().position(|x| *x.title == keyword){
        project.metadata.as_mut().unwrap().keywords.as_mut().unwrap().remove(index);
    }else{
        return ApiResult::new_error(ApiError::NotFound);
    }

    ApiResult::new_data(())
}

/// POST /api/projects/<project_id>/metadata/identifiers/
/// Add identifier to project
#[post("/api/projects/<project_id>/metadata/identifiers", data = "<identifier>")]
pub async fn add_identifier_to_project(project_id: String, mut identifier: Json<Identifier>, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<Identifier>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    if let None = identifier.id{
        identifier.id = Some(uuid::Uuid::new_v4());
    }else{
        return ApiResult::new_error(ApiError::BadRequest("Identifier is not supposed to have an id.".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();

    if let None = project.metadata{
        let new_metadata: ProjectMetadata = Default::default();
        project.metadata = Some(new_metadata);
    }

    if let None = project.metadata.as_ref().unwrap().identifiers{
        project.metadata.as_mut().unwrap().identifiers = Some(Vec::new());
    }

    if !project.metadata.as_ref().unwrap().identifiers.as_ref().unwrap().contains(&identifier){
        project.metadata.as_mut().unwrap().identifiers.as_mut().unwrap().push(identifier.clone().into_inner());
    }

    ApiResult::new_data(identifier.into_inner())
}

/// DELETE /api/projects/<project_id>/metadata/identifiers/<identifier_ic>
/// Remove identifier
#[delete("/api/projects/<project_id>/metadata/identifiers/<identifier_id>")]
pub async fn remove_identifier_from_project(project_id: String, identifier_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let identifier_id = match uuid::Uuid::parse_str(&identifier_id) {
        Ok(identifier_id) => identifier_id,
        Err(e) => {
            eprintln!("Couldn't parse identifier id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse identifier id".to_string()));
        },
    };

    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();
    if let None = project.metadata{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let None = project.metadata.as_ref().unwrap().identifiers{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let Some(index) = project.metadata.as_ref().unwrap().identifiers.as_ref().unwrap().iter().position(|x| x.id.unwrap_or_default() == identifier_id){
        project.metadata.as_mut().unwrap().identifiers.as_mut().unwrap().remove(index);
        ApiResult::new_data(())
    }else{
        ApiResult::new_error(ApiError::NotFound)
    }
}

/// PUT /api/projects/<project_id>/metadata/identifiers/<identifier_id>
/// Update identifier
#[put("/api/projects/<project_id>/metadata/identifiers/<identifier_id>", data = "<identifier>")]
pub async fn update_identifier_in_project(project_id: String, identifier_id: String, identifier: Json<Identifier>, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {

    let identifier_id = match uuid::Uuid::parse_str(&identifier_id) {
        Ok(identifier_id) => identifier_id,
        Err(e) => {
            eprintln!("Couldn't parse identifier id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse identifier id".to_string()));
        },
    };

    let mut identifier = identifier.into_inner();

    if let Some(id) = identifier.id{
        if id != identifier_id{
            return ApiResult::new_error(ApiError::BadRequest("Identifier id in url and body don't match".to_string()));
        }
    }else{
        identifier.id = Some(identifier_id);
    }

    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project_entry.write().unwrap();
    if let None = project.metadata{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let None = project.metadata.as_ref().unwrap().identifiers{
        return ApiResult::new_error(ApiError::NotFound);
    }

    if let Some(index) = project.metadata.as_ref().unwrap().identifiers.as_ref().unwrap().iter().position(|x| x.id.unwrap_or_default() == identifier_id){
        project.metadata.as_mut().unwrap().identifiers.as_mut().unwrap()[index] = identifier;
        ApiResult::new_data(())
    }else{
        ApiResult::new_error(ApiError::NotFound)
    }
}

/// GET /api/projects/<project_id>/contents
/// Returns a list of all contents (sections or toc placeholder) in the project
/// Strips out the inner content of ContentBlocks
#[get("/api/projects/<project_id>/contents")]
pub async fn get_project_contents(project_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<Vec<SectionOrToc>>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await{
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project = project.read().unwrap();

    let mut contents = Vec::new();
    for entry in project.sections.iter(){
        match entry{
            SectionOrToc::Toc => {
                contents.push(entry.clone());
            },
            SectionOrToc::Section(section) => {
                contents.push(SectionOrToc::Section(section.clone_without_contentblock_content()));
            }
        }
    }

    ApiResult::new_data(contents)
}

/// POST /api/projects/<project_id>/contents
/// Add a new section or toc placeholder to the project
#[post("/api/projects/<project_id>/contents", data = "<content>")]
pub async fn add_content(project_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>, content: Json<SectionOrToc>) -> Json<ApiResult<SectionOrToc>>{
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            eprintln!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse project id".to_string()));
        },
    };

    // Check if Section or Toc, generate uuid if section
    let mut content = content.into_inner();
    match &mut content{
        SectionOrToc::Section(section) => {
            if let None = section.id{
                section.id = Some(uuid::Uuid::new_v4());
            }
        },
        SectionOrToc::Toc => {},
    }

    let project_storage = Arc::clone(project_storage);

    let project_entry = match project_storage.get_project(&project_id, settings).await{
        Ok(project_entry) => project_entry.clone(),
        Err(_) => {
            eprintln!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    // Insert new content block at the end
    project_entry.write().unwrap().sections.push(content.clone());

    //Return inserted content block
    ApiResult::new_data(content)
}

/// PUT /api/projects/<project_id>/contents/<content_id>/move/after/<after_id>
/// Move a section or toc after another section or toc
// TODO: implement for toc
#[put("/api/projects/<project_id>/contents/<content_id>/move/after/<after_id>")]
pub async fn move_content_after(project_id: String, content_id: String, after_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let content_id = match uuid::Uuid::parse_str(&content_id) {
        Ok(content_id) => content_id,
        Err(e) => {
            eprintln!("Couldn't parse content id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content id".to_string()));
        },
    };

    let after_id = match uuid::Uuid::parse_str(&after_id) {
        Ok(after_id) => after_id,
        Err(e) => {
            eprintln!("Couldn't parse after id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse after id".to_string()));
        },
    };

    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project.write().unwrap();

    // Get section to move
    let content = match project.remove_section(&content_id){
        Some(content) => content,
        None => {
            println!("Couldn't find content with id {}", content_id);
            return ApiResult::new_error(ApiError::NotFound);
        }
    };

    // Add section after specified section
    match project.insert_section_after(&after_id, content.clone()){
        Ok(_) => ApiResult::new_data(()),
        Err(_) => {
            println!("Couldn't find content with id {}", after_id);
            //TODO re-add content to the end
            project.sections.push(SectionOrToc::Section(content));
            ApiResult::new_error(ApiError::NotFound)
        }
    }
}


/// PUT /api/projects/<project_id>/contents/<content_id>/move/child_of/<parent_id>
/// Move a section or toc to be a child of another section or toc. It will be the first child.
//TODO: Implement for toc
#[put("/api/projects/<project_id>/contents/<content_id>/move/child_of/<parent_id>")]
pub async fn move_content_child_of(project_id: String, content_id: String, parent_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let content_id = match uuid::Uuid::parse_str(&content_id) {
        Ok(content_id) => content_id,
        Err(e) => {
            eprintln!("Couldn't parse content id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content id".to_string()));
        },
    };

    let parent_id = match uuid::Uuid::parse_str(&parent_id) {
        Ok(parent_id) => parent_id,
        Err(e) => {
            eprintln!("Couldn't parse parent id: {}", e);
            return ApiResult::new_error(ApiError::BadRequest("Couldn't parse parent id".to_string()));
        },
    };

    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project.write().unwrap();

    // Get section to move
    let content = match project.remove_section(&content_id){
        Some(content) => content,
        None => {
            println!("Couldn't find content with id {}", content_id);
            return ApiResult::new_error(ApiError::NotFound);
        }
    };

    // Add section as first child of specified section
    match project.insert_section_as_first_child(&parent_id, content.clone()){
        Ok(_) => ApiResult::new_data(()),
        Err(_) => {
            println!("Couldn't find content with id {}", parent_id);
            //TODO re-add content to the end
            project.sections.push(SectionOrToc::Section(content));
            ApiResult::new_error(ApiError::NotFound)
        }
    }
}

/// GET /api/projects/<project_id>/sections/<content_id>
/// Get a section, but strip out subsections
#[get("/api/projects/<project_id>/sections/<content_path>")]
pub async fn get_section(project_id: &str, content_path: &str, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<Section>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut path = vec![];

    for part in content_path.split(":"){
        match uuid::Uuid::parse_str(part){
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    println!("Path: {:?}", path);

    if path.len() == 0{
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project = project.read().unwrap();

    let section = crate::data_storage::get_section_by_path(&project, &path);

    // TODO: check if authors and editors still exist, if not, remove them and save section
    match section{
        Ok(section) => ApiResult::new_data(section.clone_without_subsections()),
        Err(e) => ApiResult::new_error(e)
    }
}

/// PATCH /api/projects/<project_id>/sections/<content_path>
/// Patch a section, but without content (subsections / content blocks)
/// Check [PatchSection] for more information
#[patch("/api/projects/<project_id>/sections/<content_path>", data = "<section_patch>")]
pub async fn update_section(project_id: String, content_path: String, section_patch: Json<PatchSection>, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>, data_storage: &State<Arc<DataStorage>>) -> Json<ApiResult<Section>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut path = vec![];

    for part in content_path.split(":"){
        match uuid::Uuid::parse_str(part){
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    if path.len() == 0{
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project.write().unwrap();

    let section = crate::data_storage::get_section_by_path_mut(&mut project, &path);

    match section{
        Ok(section) => {
            let mut new_section_data = section.patch(section_patch.into_inner());
            // Check if new section data is valid
            // Check authors
            for author in new_section_data.metadata.authors.iter(){
                if !data_storage.person_exists(author){
                    return ApiResult::new_error(ApiError::BadRequest(format!("Author {} does not exist", author)));
                }
            }

            // Check editors
            for editor in new_section_data.metadata.editors.iter(){
                if !data_storage.person_exists(editor){
                    return ApiResult::new_error(ApiError::BadRequest(format!("Editor {} does not exist", editor)));
                }
            }

            // Remove duplicants
            new_section_data.metadata.authors.sort_unstable();
            new_section_data.metadata.authors.dedup();
            new_section_data.metadata.editors.sort_unstable();
            new_section_data.metadata.editors.dedup();

            // Add ids for identifiers
            for identifier in new_section_data.metadata.identifiers.iter_mut(){
                if identifier.id.is_none(){
                    identifier.id = Some(uuid::Uuid::new_v4());
                }
            }


            // Set last changed to now
            new_section_data.metadata.last_changed = Some(chrono::Utc::now().naive_utc());

            *section = new_section_data.clone();

            ApiResult::new_data(new_section_data)
        },
        Err(e) => ApiResult::new_error(e)
    }
}

/// DELETE /api/projects/<project_id>/sections/<content_path>
/// Delete a section including all subsections and content blocks
#[delete("/api/projects/<project_id>/sections/<content_path>")]
pub async fn delete_section(project_id: String, content_path: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut path = vec![];

    for part in content_path.split(":"){
        match uuid::Uuid::parse_str(part){
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    if path.len() == 0{
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project.write().unwrap();

    match project.remove_section(path.last().unwrap()){
        Some(_) => ApiResult::new_data(()),
        None => ApiResult::new_error(ApiError::NotFound)
    }
}

/// GET /api/projects/<project_id>/sections/<content_path>/content_blocks
/// Get all content blocks in a section
#[get("/api/projects/<project_id>/sections/<content_path>/content_blocks")]
pub async fn get_content_blocks_in_section(project_id: String, content_path: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<Vec<ContentBlock>>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut path = vec![];

    for part in content_path.split(":"){
        match uuid::Uuid::parse_str(part){
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    if path.len() == 0{
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project = project.read().unwrap();

    let section = crate::data_storage::get_section_by_path(&project, &path);

    match section{
        Ok(section) => {
            let mut content_blocks = Vec::new();
            for child in section.children.iter(){
                match child{
                    crate::projects::SectionContent::ContentBlock(block) => {
                        content_blocks.push(block.clone());
                    },
                    _ => {},
                }
            }
            ApiResult::new_data(content_blocks)
        },
        Err(e) => ApiResult::new_error(e)
    }
}

/// GET /api/projects/<project_id>/sections/<content_path>/content_blocks/<content_block_id>
/// Get a single content block
#[get("/api/projects/<project_id>/sections/<content_path>/content_blocks/<content_block_id>")]
pub async fn get_content_block(project_id: String, content_path: String, content_block_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<ContentBlock>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let content_block_id = match uuid::Uuid::parse_str(&content_block_id) {
        Ok(content_block_id) => content_block_id,
        Err(e) => {
            println!("Couldn't parse content block id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut path = vec![];

    for part in content_path.split(":"){
        match uuid::Uuid::parse_str(part){
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    if path.len() == 0{
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let project = project.read().unwrap();

    let section = crate::data_storage::get_section_by_path(&project, &path);

    match section{
        Ok(section) => {
            for child in section.children.iter(){
                match child{
                    crate::projects::SectionContent::ContentBlock(block) => {
                        if block.id.unwrap_or_default() == content_block_id{
                            return ApiResult::new_data(block.clone());
                        }
                    },
                    _ => {},
                }
            }
            ApiResult::new_error(
                ApiError::NotFound
            )
        },
        Err(e) => ApiResult::new_error(e)
    }
}

/// DELETE /api/projects/<project_id>/sections/<content_path>/content_blocks/<content_block_id>
/// Delete a content block
#[delete("/api/projects/<project_id>/sections/<content_path>/content_blocks/<content_block_id>")]
pub async fn delete_content_block(project_id: String, content_path: String, content_block_id: String, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let content_block_id = match uuid::Uuid::parse_str(&content_block_id) {
        Ok(content_block_id) => content_block_id,
        Err(e) => {
            println!("Couldn't parse content block id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut path = vec![];

    for part in content_path.split(":"){
        match uuid::Uuid::parse_str(part){
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    if path.len() == 0{
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project.write().unwrap();

    let section = crate::data_storage::get_section_by_path_mut(&mut project, &path);

    match section{
        Ok(section) => {
            let mut index = None;
            for (i, child) in section.children.iter().enumerate(){
                match child{
                    crate::projects::SectionContent::ContentBlock(block) => {
                        if block.id.unwrap_or_default() == content_block_id{
                            index = Some(i);
                            break;
                        }
                    },
                    _ => {},
                }
            }

            if let Some(index) = index{
                section.children.remove(index);
                ApiResult::new_data(())
            }else{
                ApiResult::new_error(ApiError::NotFound)
            }
        },
        Err(e) => ApiResult::new_error(e)
    }
}

/// POST /api/projects/<project_id>/sections/<content_path>/content_blocks
/// Add a new content block to a section
#[post("/api/projects/<project_id>/sections/<content_path>/content_blocks", data = "<content_block>")]
pub async fn add_content_block_to_section(project_id: String, content_path: String, content_block: Json<ContentBlock>, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<ContentBlock>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut new_block = content_block.into_inner();
    if new_block.id.is_some(){
        return ApiResult::new_error(ApiError::BadRequest("Creating blocks with pre-defined id is not permitted.".to_string()));
    }
    new_block.id = Some(uuid::Uuid::new_v4());


    let mut path = vec![];

    for part in content_path.split(":"){
        match uuid::Uuid::parse_str(part){
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    if path.len() == 0{
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project.write().unwrap();

    let section = crate::data_storage::get_section_by_path_mut(&mut project, &path);

    match section{
        Ok(section) => {
            section.children.push(crate::projects::SectionContent::ContentBlock(new_block.clone()));
            ApiResult::new_data(new_block)
        },
        Err(e) => ApiResult::new_error(e)
    }
}

/// PUT /api/projects/<project_id>/sections/<content_path>/content_blocks/<content_block_id>
/// Update a content block
#[put("/api/projects/<project_id>/sections/<content_path>/content_blocks/<content_block_id>", data = "<content_block>")]
pub async fn update_contentblock(project_id: String, content_block_id: String, content_path: String, content_block: Json<ContentBlock>, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    println!("Content Block: {:?}", content_block.clone());
    //TODO: filter out \u{200b} (zero width space) from content
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let content_block_id = match uuid::Uuid::parse_str(&content_block_id) {
        Ok(content_block_id) => content_block_id,
        Err(e) => {
            println!("Couldn't parse content block id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    if content_block_id != content_block.id.unwrap_or_default(){
        return ApiResult::new_error(ApiError::BadRequest("Content block id in url and body don't match".to_string()));
    }

    let mut path = vec![];

    for part in content_path.split(":"){
        match uuid::Uuid::parse_str(part){
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    if path.len() == 0{
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project.write().unwrap();

    let section = crate::data_storage::get_section_by_path_mut(&mut project, &path);

    match section{
        Ok(section) => {
            for child in section.children.iter_mut(){
                match child{
                    crate::projects::SectionContent::ContentBlock(block) => {
                        if block.id.unwrap_or_default() == content_block_id{
                            *block = content_block.into_inner();
                            return ApiResult::new_data(());
                        }
                    },
                    _ => {},
                }
            }
            ApiResult::new_error(
                ApiError::NotFound
            )
        },
        Err(e) => ApiResult::new_error(e)
    }
}

#[derive(Deserialize, Serialize, Debug, Encode, Decode, Clone, PartialEq, Default)]
struct PatchContentBlock{
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    #[bincode(with_serde)]
    pub id: Option<Option<uuid::Uuid>>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    #[bincode(with_serde)]
    pub revision_id: Option<Option<uuid::Uuid>>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub content: Option<Option<PatchInnerContentBlock>>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "::serde_with::rust::double_option")]
    pub css_classes: Option<Option<Vec<String>>>,
}
impl Patch<PatchContentBlock, ContentBlock> for ContentBlock{
    fn patch(&mut self, patch: PatchContentBlock) -> ContentBlock {
        let new_block = self.clone();

        if let Some(id) = patch.id{
            self.id = id;
        }
        if let Some(revision_id) = patch.revision_id{
            self.revision_id = revision_id;
        }
        if let Some(content) = patch.content{
            match content{
                Some(content) => {
                    match self.content{
                        Some(ref mut inner_content) => {
                            let new = inner_content.patch(content);
                            self.content = Some(new);
                        },
                        None => { //TODO: do not panic
                            panic!("Content block has no content, but patch has content");
                        }
                    }
                },
                None => {
                    self.content = None;
                }

            }
        }
        if let Some(css_classes) = patch.css_classes{
            self.css_classes = css_classes;
        }
        self.clone()
    }
}

/// PATCH /api/projects/<project_id>/sections/<content_path>/content_blocks/<content_block_id>
/// Update a content block by patching it
#[patch("/api/projects/<project_id>/sections/<content_path>/content_blocks/<content_block_id>", data = "<content_block_patch>")]
pub async fn patch_contentblock(project_id: String, content_block_id: String, content_path: String, content_block_patch: Json<PatchContentBlock>, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<ContentBlock>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let content_block_id = match uuid::Uuid::parse_str(&content_block_id) {
        Ok(content_block_id) => content_block_id,
        Err(e) => {
            println!("Couldn't parse content block id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut path = vec![];

    for part in content_path.split(":") {
        match uuid::Uuid::parse_str(part) {
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    if path.len() == 0 {
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project.write().unwrap();

    let section = crate::data_storage::get_section_by_path_mut(&mut project, &path);

    match section {
        Ok(section) => {
            for child in section.children.iter_mut() {
                match child {
                    crate::projects::SectionContent::ContentBlock(block) => {
                        if block.id.unwrap_or_default() == content_block_id {
                            if block.content.is_none(){
                                return ApiResult::new_error(ApiError::BadRequest("Content block has no content. Can't patch.".to_string()));
                            }
                            let mut new_block_data = block.patch(content_block_patch.into_inner());

                            // Check blockdata
                            if new_block_data.id.is_none(){
                                return ApiResult::new_error(ApiError::BadRequest("Content block id is missing".to_string()));
                            }
                            //TODO: check if revision_id is none

                            *block = new_block_data.clone();
                            return ApiResult::new_data(new_block_data);
                        }
                    },
                    _ => {},
                }
            }
            ApiResult::new_error(
                ApiError::NotFound
            )
        },
        Err(e) => ApiResult::new_error(e)
    }
}

#[derive(Deserialize)]
pub struct MoveContentBlock{
    pub content_block_id: uuid::Uuid,
    /// If None, the content block will be moved to the beginning of the section
    pub insert_after: Option<uuid::Uuid>,
}

/// POST /api/projects/<project_id>/sections/<content_path>/content_blocks/move
/// Move a content block to another position
#[post("/api/projects/<project_id>/sections/<content_path>/content_blocks/move", data = "<move_data>")]
pub async fn move_contentblock(project_id: String, content_path: String, move_data: Json<MoveContentBlock>, _session: Session, settings: &State<Settings>, project_storage: &State<Arc<ProjectStorage>>) -> Json<ApiResult<()>> {
    let project_id = match uuid::Uuid::parse_str(&project_id) {
        Ok(project_id) => project_id,
        Err(e) => {
            println!("Couldn't parse project id: {}", e);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut path = vec![];

    for part in content_path.split(":") {
        match uuid::Uuid::parse_str(part) {
            Ok(part) => path.push(part),
            Err(e) => {
                println!("Couldn't parse content path: {}", e);
                return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
            }
        }
    }

    if path.len() == 0 {
        println!("Couldn't parse content path: path is empty");
        return ApiResult::new_error(ApiError::BadRequest("Couldn't parse content path".to_string()));
    }

    let project_storage = Arc::clone(project_storage);

    let project = match project_storage.get_project(&project_id, settings).await {
        Ok(project) => project,
        Err(_) => {
            println!("Couldn't get project with id {}", project_id);
            return ApiResult::new_error(ApiError::NotFound);
        },
    };

    let mut project = project.write().unwrap();

    let section = match crate::data_storage::get_section_by_path_mut(&mut project, &path){
        Ok(section) => section,
        Err(e) => {
            return ApiResult::new_error(e);
        }
    };

    // Find content block
    let mut content_block_pos = None;
    for (i, block) in section.children.iter().enumerate(){
        if let crate::projects::SectionContent::ContentBlock(block) = block{
            if block.id.unwrap_or_default() == move_data.content_block_id{
                content_block_pos = Some(i);
                break;
            }
        }
    }
    let content_block_pos = match content_block_pos{
        Some(pos) => pos,
        None => {
            return ApiResult::new_error(ApiError::NotFound);
        }
    };

    // Remove block from old position
    let block = section.children.remove(content_block_pos);

    // Find new target and insert block
    let mut insert_pos = None;
    match move_data.insert_after{
        Some(insert_after) => {
            for (i, block) in section.children.iter().enumerate(){
                if let crate::projects::SectionContent::ContentBlock(block) = block{
                    if block.id.unwrap_or_default() == insert_after{
                        insert_pos = Some(i+1);
                        break;
                    }
                }
            }
        },
        None => {
            insert_pos = Some(0);
        }
    }

    let insert_pos = match insert_pos{
        Some(pos) => pos,
        None => {
            // Re-add content block to old position
            section.children.insert(content_block_pos, block);
            return ApiResult::new_error(ApiError::NotFound);
        }
    };
    section.children.insert(insert_pos, block);
    ApiResult::new_data(())

}