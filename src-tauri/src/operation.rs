use serde::Serialize;
use tauri::{Emitter, Window};

use crate::error::AppError;

/// Operation progress events (`operation_<id>`): started/finished/failed
/// per step. Steps for installs: download → install → pairing.
pub struct Operation<'a> {
    id: String,
    window: &'a Window,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationUpdate<'a> {
    update_type: &'a str,
    step_id: &'a str,
    extra_details: Option<AppError>,
}

impl<'a> Operation<'a> {
    pub fn new(id: String, window: &'a Window) -> Self {
        Self { id, window }
    }

    pub fn move_on(&self, old_id: &str, new_id: &str) -> Result<(), AppError> {
        self.complete(old_id)?;
        self.start(new_id)
    }

    pub fn start(&self, id: &str) -> Result<(), AppError> {
        self.emit("started", id, None)
    }

    pub fn complete(&self, id: &str) -> Result<(), AppError> {
        self.emit("finished", id, None)
    }

    pub fn fail<T>(&self, id: &str, error: AppError) -> Result<T, AppError> {
        self.emit("failed", id, Some(error.clone()))?;
        Err(error)
    }

    pub fn fail_if_err<T>(&self, id: &str, res: Result<T, AppError>) -> Result<T, AppError> {
        match res {
            Ok(t) => Ok(t),
            Err(e) => self.fail::<T>(id, e),
        }
    }

    fn emit(&self, kind: &str, step: &str, details: Option<AppError>) -> Result<(), AppError> {
        self.window
            .emit(
                &format!("operation_{}", self.id),
                OperationUpdate { update_type: kind, step_id: step, extra_details: details },
            )
            .map_err(|e| AppError::OperationUpdate(e.to_string()))
    }
}
