use std::collections::HashMap;
use std::{error, fmt, mem};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicU64;
use serde::Serialize;
use crate::data_storage::{DataStorage, ProjectData};
use crate::export::preprocessing::{prepare_project, render_project};
use crate::settings::Settings;

#[derive(Default, Clone, Serialize)]
pub enum RenderingStatus{
    #[default]
    Queued,
    Preparing,
    Running,
    Finished,
    Failed(RenderingError),
}

#[derive(Default)]
pub struct RenderingRequest{
    pub rendering_id: uuid::Uuid,
    pub status: RenderingStatus,
    pub project_data: Option<ProjectData>,
}

pub struct RenderingManager{
    pub settings: Settings,
    pub data_storage: Arc<DataStorage>,
    pub requests_archive: RwLock<HashMap<uuid::Uuid, RwLock<RenderingRequest>>>,
    pub rendering_requests: RwLock<Vec<RwLock<RenderingRequest>>>,
}

#[derive(Debug, Clone, Serialize)]
pub enum RenderingError{
    NoProjectData,
    ProjectMetadataMissing,
    ErrorLoadingTemplate(String),
    VivliostyleError(String),
    ErrorCopyingTemplate(String),
    IoError(String),
}

impl fmt::Display for RenderingError{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self{
            RenderingError::NoProjectData => write!(f, "No project data found in rendering request"),
            RenderingError::ProjectMetadataMissing => write!(f, "Project metadata missing"),
            RenderingError::ErrorLoadingTemplate(ref e) => write!(f, "Error loading template: {}", e),
            RenderingError::VivliostyleError(ref e) => write!(f, "Vivliostyle error: {}", e),
            RenderingError::ErrorCopyingTemplate(ref e) => write!(f, "Error copying template files: {}", e),
            RenderingError::IoError(ref e) => write!(f, "I/O Error occurred: {}", e),
        }
    }
}

impl error::Error for RenderingError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            RenderingError::NoProjectData => None,
            RenderingError::ProjectMetadataMissing => None,
            RenderingError::ErrorLoadingTemplate(_) => None,
            RenderingError::VivliostyleError(_) => None,
            RenderingError::ErrorCopyingTemplate(_) => None,
            RenderingError::IoError(_) => None,
        }
    }
}

impl RenderingManager{
    pub async fn start(settings: Settings, data_storage: Arc<DataStorage>) -> Arc<RenderingManager>{
        let rendering_manager = RenderingManager{
            settings,
            data_storage,
            requests_archive: RwLock::new(HashMap::new()),
            rendering_requests: RwLock::new(Vec::new()),
        };

        let rendering_manager = Arc::new(rendering_manager);
        let rendering_manager_cpy = rendering_manager.clone();


        // Start thread that checks for new rendering requests and starts them in a new thread
        tokio::spawn(async move {
            let running_threads: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

            //TODO: kill hanging threads
            loop{
                // Check if there are any new rendering requests

                let rendering_requests_len = rendering_manager_cpy.rendering_requests.read().unwrap().len();
                if rendering_requests_len > 0 && rendering_manager_cpy.settings.max_rendering_threads > running_threads.load(std::sync::atomic::Ordering::Relaxed){
                    println!("Starting new rendering thread for request...");
                    {
                        let mut rendering_requests = rendering_manager_cpy.rendering_requests.write().unwrap();
                        running_threads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                        // Move the rendering request out of the vector, put it into the archive and start rendering
                        let request_id = rendering_requests.first().unwrap().read().unwrap().rendering_id;
                        rendering_manager_cpy.requests_archive.write().unwrap().insert(request_id, rendering_requests.pop().unwrap());

                        let rendering_manager_cpy2 = rendering_manager_cpy.clone();
                        let rendering_manager_cpy3 = rendering_manager_cpy.clone();

                        let running_threads_clone = Arc::clone(&running_threads);

                        // Start rendering in a new thread
                        tokio::spawn(async move {
                            match Self::render(rendering_manager_cpy2, request_id){
                                Ok(_) => {
                                    let mut storage = rendering_manager_cpy3.requests_archive.write().unwrap();
                                    let mut rendering_request = storage.get_mut(&request_id).unwrap().write().unwrap();
                                    rendering_request.status = RenderingStatus::Finished;
                                }
                                Err(e) => {
                                    let mut storage = rendering_manager_cpy3.requests_archive.write().unwrap();
                                    let mut rendering_request = storage.get_mut(&request_id).unwrap().write().unwrap();
                                    rendering_request.status = RenderingStatus::Failed(e);
                                }
                            }
                            running_threads_clone.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                        });
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        rendering_manager.clone()
    }
    fn render(rendering_manager: Arc<RenderingManager>, request_id: uuid::Uuid) -> Result<(), RenderingError>{
        let project_data: ProjectData = { // Introduction of a new scope to drop the lock on the request
            let mut storage = rendering_manager.requests_archive.write().unwrap();
            let mut rendering_request = storage.get_mut(&request_id).unwrap().write().unwrap();
            rendering_request.status = RenderingStatus::Preparing;
            match mem::take(&mut rendering_request.project_data) {
                Some(project_data) => project_data,
                None => {
                    rendering_request.status = RenderingStatus::Failed(RenderingError::NoProjectData);
                    return Err(RenderingError::NoProjectData);
                }
            }
        };

        let template_id = project_data.template_id;

        let binding = format!("{}/temp/{}", rendering_manager.settings.data_path, request_id);
        let temp_dir = Path::new(&binding);
        if temp_dir.exists(){
            std::fs::remove_dir_all(temp_dir).unwrap();
        }

        std::fs::create_dir_all(temp_dir).unwrap();

        // Prepare project
        let prepared_project = prepare_project(project_data, rendering_manager.data_storage.clone())?;

        // Update project status
        {
            let mut storage = rendering_manager.requests_archive.write().unwrap();
            let mut rendering_request = storage.get_mut(&request_id).unwrap().write().unwrap();
            rendering_request.status = RenderingStatus::Running;
        }

        // Render
        match render_project(prepared_project, template_id, temp_dir, &rendering_manager.settings){
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    pub fn add_rendering_request(&self, project_data: ProjectData) -> uuid::Uuid{
        let rendering_id = uuid::Uuid::new_v4();
        let rendering_request = RenderingRequest{
            rendering_id,
            status: RenderingStatus::Queued,
            project_data: Some(project_data),
        };

        self.rendering_requests.write().unwrap().push(RwLock::new(rendering_request));
        rendering_id
    }

    pub fn get_rendering_request_status(&self, rendering_id: uuid::Uuid) -> Option<RenderingStatus>{
        let storage = self.requests_archive.read().unwrap();
        match storage.get(&rendering_id){
            Some(request) => {
                let request = request.read().unwrap();
                Some(request.status.clone())
            }
            None => {
                // Check if the request is still in the rendering_requests vector
                let storage = self.rendering_requests.read().unwrap();
                for request in storage.iter(){
                    let request = request.read().unwrap();
                    if request.rendering_id == rendering_id{
                        return Some(request.status.clone());
                    }
                }
                None
            }
        }
    }
}