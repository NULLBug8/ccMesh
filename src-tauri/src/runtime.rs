use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Clone)]
pub struct AppHandle {
    inner: Arc<AppContext>,
}

struct AppContext {
    data_dir: PathBuf,
    resource_dir: PathBuf,
    state: OnceLock<Arc<AppState>>,
}

#[derive(Clone)]
pub struct AppPaths {
    data_dir: PathBuf,
    resource_dir: PathBuf,
}

pub struct State<'a, T: ?Sized>(&'a T);

impl<T: ?Sized> Copy for State<'_, T> {}

impl<T: ?Sized> Clone for State<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl AppHandle {
    pub fn new(data_dir: PathBuf, resource_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(AppContext {
                data_dir,
                resource_dir,
                state: OnceLock::new(),
            }),
        }
    }

    pub fn set_state(&self, state: Arc<AppState>) -> AppResult<()> {
        self.inner
            .state
            .set(state)
            .map_err(|_| AppError::Config("application state is already initialized".into()))
    }

    pub fn state(&self) -> State<'_, AppState> {
        State(
            self.inner
                .state
                .get()
                .expect("application state is not initialized")
                .as_ref(),
        )
    }

    pub fn path(&self) -> AppPaths {
        AppPaths {
            data_dir: self.inner.data_dir.clone(),
            resource_dir: self.inner.resource_dir.clone(),
        }
    }

    pub fn emit<T: serde::Serialize>(&self, event: &str, payload: T) -> AppResult<()> {
        crate::modules::web_admin::bridge::emit(event, &payload);
        Ok(())
    }
}

impl AppPaths {
    pub fn app_data_dir(&self) -> AppResult<PathBuf> {
        Ok(self.data_dir.clone())
    }

    pub fn resource_dir(&self) -> AppResult<PathBuf> {
        Ok(self.resource_dir.clone())
    }
}

impl<'a, T: ?Sized> State<'a, T> {
    pub fn new(value: &'a T) -> Self {
        Self(value)
    }
}

impl<T: ?Sized> std::ops::Deref for State<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
