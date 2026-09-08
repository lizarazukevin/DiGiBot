//! Service layer for `GitHub` App installation repository events.

use crate::error::AppError;
use crate::github::api::pull_requests::split_repo;
use crate::github::webhook::events::installation_repositories::InstallationRepositoriesPayload;
use crate::models::subscription::SubscriptionStore;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationRepositoriesAction {
	Added,
	Removed,
	Other,
}

pub struct InstallationRepositoriesRequest {
	pub action: InstallationRepositoriesAction,
	pub repositories_removed: Vec<String>,
}

impl InstallationRepositoriesRequest {
	pub(crate) fn from_payload(payload: InstallationRepositoriesPayload) -> Self {
		Self {
			action: match payload.action.as_str() {
				"added" => InstallationRepositoriesAction::Added,
				"removed" => InstallationRepositoriesAction::Removed,
				_ => InstallationRepositoriesAction::Other,
			},
			repositories_removed: payload
				.repositories_removed
				.into_iter()
				.map(|repo| repo.full_name)
				.collect(),
		}
	}
}

pub struct InstallationRepositoriesService {
	sub_store: Arc<dyn SubscriptionStore>,
}

impl InstallationRepositoriesService {
	pub fn new(sub_store: Arc<dyn SubscriptionStore>) -> Self {
		Self { sub_store }
	}

	/// React to an installation repository lifecycle event.
	///
	/// For removed: removes all subscriptions for every repository that was
	/// removed from the installation.
	/// For added: logs the addition (subscriptions are created lazily on
	/// first event via `/subscribe`).
	pub async fn handle(&self, req: InstallationRepositoriesRequest) -> Result<(), AppError> {
		match req.action {
			InstallationRepositoriesAction::Added => {
				info!("repositories added to installation");
			}
			InstallationRepositoriesAction::Removed => {
				info!("repositories removed from installation, cleaning up subscriptions");

				for repo in &req.repositories_removed {
					let (owner, project) = split_repo(repo)?;
					self.sub_store
						.delete_all_by_owner_project(owner, project)
						.await?;
				}
			}
			InstallationRepositoriesAction::Other => {}
		}

		Ok(())
	}
}
