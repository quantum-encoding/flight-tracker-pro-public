use git2::{Repository, Signature, IndexAddOption, RepositoryInitOptions};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use chrono::Utc;

/// Represents a workflow checkpoint in git
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub commit_hash: String,
    pub message: String,
    pub timestamp: String,
    pub workflow_id: String,
}

/// Git-based checkpoint manager for workflow versioning
pub struct CheckpointManager {
    repo_path: PathBuf,
    workflow_id: String,
}

impl CheckpointManager {
    /// Initialize a new git repository for a workflow
    pub fn init(workflow_id: &str) -> Result<Self> {
        let repo_path = Self::get_workflow_repo_path(workflow_id)?;

        // Initialize git repo if it doesn't exist
        let repo = if repo_path.exists() {
            Repository::open(&repo_path)
                .context("Failed to open existing workflow repository")?
        } else {
            std::fs::create_dir_all(&repo_path)
                .context("Failed to create workflow directory")?;

            let mut opts = RepositoryInitOptions::new();
            opts.bare(false);
            opts.initial_head("main");

            Repository::init_opts(&repo_path, &opts)
                .context("Failed to initialize git repository")?
        };

        // Always create/update .gitignore
        let gitignore_path = repo_path.join(".gitignore");
        std::fs::write(&gitignore_path, "*.tmp\n*.log\n")?;

        // Always create/update README with workflow metadata
        let readme_path = repo_path.join("README.md");
        let readme_content = format!(
            "# Workflow Execution: {}\n\nInitialized: {}\n\nThis repository tracks all executions and state changes for this workflow.\n",
            workflow_id,
            Utc::now().to_rfc3339()
        );
        std::fs::write(&readme_path, readme_content)?;

        // Ensure initial commit exists
        let needs_initial_commit = repo.is_empty()? || repo.head().is_err();

        if needs_initial_commit {
            let mut index = repo.index()?;
            index.add_all(["."].iter(), IndexAddOption::DEFAULT, None)?;
            index.write()?;

            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            let sig = Self::create_signature()?;

            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Initial commit: Workflow repository initialized",
                &tree,
                &[],
            )?;

            tracing::info!("Created initial commit for workflow {}", workflow_id);
        }

        Ok(Self {
            repo_path,
            workflow_id: workflow_id.to_string(),
        })
    }

    /// Create a checkpoint with workflow state
    pub fn create_checkpoint(&self, message: &str, data: &str) -> Result<Checkpoint> {
        let repo = Repository::open(&self.repo_path)?;

        // Write workflow state to file
        let state_path = self.repo_path.join("workflow_state.json");
        std::fs::write(&state_path, data)?;

        // Create execution log entry
        let log_path = self.repo_path.join("execution.log");
        let log_entry = format!(
            "[{}] {}\n",
            Utc::now().to_rfc3339(),
            message
        );
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?
            .write_all(log_entry.as_bytes())?;

        // Stage changes
        let mut index = repo.index()?;
        index.add_all(["."].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Create tree
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // Get parent commit
        let parent_commit = repo.head()?.peel_to_commit()?;

        // Create commit
        let sig = Self::create_signature()?;
        let commit_id = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&parent_commit],
        )?;

        Ok(Checkpoint {
            commit_hash: commit_id.to_string(),
            message: message.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            workflow_id: self.workflow_id.clone(),
        })
    }

    /// Get checkpoint history
    pub fn get_history(&self) -> Result<Vec<Checkpoint>> {
        let repo = Repository::open(&self.repo_path)?;

        // Check if repository is empty (no commits yet)
        if repo.is_empty()? {
            return Ok(Vec::new());
        }

        // Check if HEAD exists
        if repo.head().is_err() {
            return Ok(Vec::new());
        }

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut checkpoints = Vec::new();

        for oid in revwalk.take(50) {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            checkpoints.push(Checkpoint {
                commit_hash: oid.to_string(),
                message: commit.message().unwrap_or("").to_string(),
                timestamp: chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
                    .unwrap_or_default()
                    .to_rfc3339(),
                workflow_id: self.workflow_id.clone(),
            });
        }

        Ok(checkpoints)
    }

    /// Get workflow state at a specific checkpoint
    pub fn get_state_at_checkpoint(&self, commit_hash: &str) -> Result<String> {
        let repo = Repository::open(&self.repo_path)?;
        let oid = git2::Oid::from_str(commit_hash)?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let entry = tree.get_name("workflow_state.json")
            .context("workflow_state.json not found in commit")?;
        let object = entry.to_object(&repo)?;
        let blob = object.as_blob()
            .context("workflow_state.json is not a blob")?;

        let content = std::str::from_utf8(blob.content())?;
        Ok(content.to_string())
    }

    /// Get the repository path for a workflow
    fn get_workflow_repo_path(workflow_id: &str) -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .context("Failed to get home directory")?;

        let workflows_dir = home_dir
            .join(".flight_tracker_pro")
            .join("workflows")
            .join(workflow_id);

        Ok(workflows_dir)
    }

    /// Create a git signature
    fn create_signature<'a>() -> Result<Signature<'a>> {
        Signature::now(
            "Flight Tracker Pro",
            "workflow@flighttracker.local",
        ).context("Failed to create git signature")
    }

    /// Get repository path
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
}

use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_init() {
        let workflow_id = format!("test-{}", uuid::Uuid::new_v4());
        let manager = CheckpointManager::init(&workflow_id);
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert!(manager.repo_path().exists());
    }

    #[test]
    fn test_create_checkpoint() {
        let workflow_id = format!("test-{}", uuid::Uuid::new_v4());
        let manager = CheckpointManager::init(&workflow_id).unwrap();

        let checkpoint = manager.create_checkpoint(
            "Test checkpoint",
            r#"{"status": "running"}"#,
        );

        assert!(checkpoint.is_ok());
        let checkpoint = checkpoint.unwrap();
        assert!(!checkpoint.commit_hash.is_empty());
    }

    #[test]
    fn test_checkpoint_history() {
        let workflow_id = format!("test-{}", uuid::Uuid::new_v4());
        let manager = CheckpointManager::init(&workflow_id).unwrap();

        manager.create_checkpoint("Checkpoint 1", r#"{"step": 1}"#).unwrap();
        manager.create_checkpoint("Checkpoint 2", r#"{"step": 2}"#).unwrap();

        let history = manager.get_history().unwrap();
        assert!(history.len() >= 2);
    }
}
