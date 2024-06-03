use std::{fmt, ops::Range, path::Path};

use async_trait::async_trait;
use opendal::Operator;
use proto::semantics::GcsCollectorConfig;
use tokio::io::{AsyncRead, AsyncWriteExt};

use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};

#[derive(Clone)]
pub struct OpendalStorage {
	op: Operator,
	_bucket: Option<String>,
}

impl fmt::Debug for OpendalStorage {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter
			.debug_struct("OpendalStorage")
			.field("operator", &self.op.info())
			.finish()
	}
}

impl OpendalStorage {
	/// Create a new google cloud storage.
	pub fn new_google_cloud_storage(
		cfg: opendal::services::Gcs,
		bucket: Option<String>,
	) -> Result<Self, SourceError> {
		let op = Operator::new(cfg)?.finish();
		Ok(Self { op, _bucket: bucket })
	}
}

#[async_trait]
impl Source for OpendalStorage {
	async fn list_files(&self, path: &Path) -> SourceResult<Vec<String>> {
		let path_str = path.to_string_lossy();
		let mut files_list = Vec::new();


		let objs = self.op.list(&path_str).await.map_err(|e| {
			println!("Failed to list objects: {:?}", e);
			e
		})?;
		for obj in objs {
			files_list.push(obj.path().to_owned());
		}
		Ok(files_list)
	}
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		self.op.check().await?;
		Ok(())
	}

	async fn copy_to(&self, path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let path = path.as_os_str().to_string_lossy();
		let mut storage_reader = self.op.reader(&path).await?;
		tokio::io::copy(&mut storage_reader, output).await?;
		output.flush().await?;
		Ok(())
	}

	async fn get_slice(&self, path: &Path, range: Range<usize>) -> SourceResult<Vec<u8>> {
		let path = path.as_os_str().to_string_lossy();
		let range = range.start as u64..range.end as u64;
		let storage_content = self.op.read_with(&path).range(range).await?;

		Ok(storage_content)
	}

	async fn get_slice_stream(
		&self,
		path: &Path,
		range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let path = path.as_os_str().to_string_lossy();
		let range = range.start as u64..range.end as u64;
		let storage_reader = self.op.reader_with(&path).range(range).await?;

		Ok(Box::new(storage_reader))
	}

	async fn get_all(&self, path: &Path) -> SourceResult<Vec<u8>> {
		// let path = path.as_os_str().to_string_lossy();
		let path_str = path.to_string_lossy();
		let storage_content = self.op.read(&path_str).await?;

		Ok(storage_content)
	}

	async fn file_num_bytes(&self, path: &Path) -> SourceResult<u64> {
		let path = path.as_os_str().to_string_lossy();
		let meta = self.op.stat(&path).await?;
		Ok(meta.content_length())
	}
}

impl From<opendal::Error> for SourceError {
	fn from(err: opendal::Error) -> Self {
		match err.kind() {
			opendal::ErrorKind::NotFound => SourceError::new(
				SourceErrorKind::NotFound,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
			opendal::ErrorKind::PermissionDenied => SourceError::new(
				SourceErrorKind::Unauthorized,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
			opendal::ErrorKind::ConfigInvalid => SourceError::new(
				SourceErrorKind::NotSupported,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
			_ => SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Opendal error: {:?}", err).into(),
			),
		}
	}
}

pub fn get_gcs_storage(gcs_config: GcsCollectorConfig) -> Result<OpendalStorage, SourceError> {
	// let credentials: Value = serde_json::from_str(&gcs_config.credentials).expect("Failed to parse credentials");
	let mut cfg = opendal::services::Gcs::default();
	cfg.credential(&gcs_config.credentials);
	cfg.bucket(&gcs_config.bucket);
	OpendalStorage::new_google_cloud_storage(cfg, Some(gcs_config.bucket))
}


#[cfg(test)]
mod tests {

use super::*;

	#[tokio::test]
	async fn test_gcs_collector() {
		// Configure the GCS collector config with a mock credential
		let gcs_config = GcsCollectorConfig {
		    bucket: "querent-testing-api".to_string(),
            credentials: "eyJ0eXBlIjogInNlcnZpY2VfYWNjb3VudCIsIAoJCQkicHJvamVjdF9pZCI6ICJxdWVyZW50LTEiLCAKCQkJInByaXZhdGVfa2V5X2lkIjogIjQ2MWRhMzcxNzY5ODc5NDAyNTgxMDliNDExZDU5MmU3ZGFlZTlhOTgiLCAKCQkJInByaXZhdGVfa2V5IjogIi0tLS0tQkVHSU4gUFJJVkFURSBLRVktLS0tLVxuTUlJRXZRSUJBREFOQmdrcWhraUc5dzBCQVFFRkFBU0NCS2N3Z2dTakFnRUFBb0lCQVFEUVVYclRIYlFFLy91ZFxuMG1VRWpGblkyQU9wSW1oSkdDeGw1bWZpSWloVU5sSzNaeEhHdEIyQlI3UXJiKzZYOUZCVHBieFZabW5hK05pQlxuRUpSdUpDWlJkaUk2NG44Nk4zQ0hVaVFGZWtGaDIzcm1xUjc1S3RLWkVDSEJBaGxSaEVzSXZ1Qmc1czVpYm9sTlxuVXVGZzJhWnNKbkNyUnVKK2tZSjJZd2xXaTU4YTFLR3dNNksrSFVubEQ3UGhqc2JQOUtGd3dGL1kzcm45TEJUZFxuRE5NTUp3OHJUUEZsaUtvVGxDcHBGRWcrY0FIZzBRZ3p0Q3U2ZkYzWURwYTZlUU16b0RLQjFWQlNIUlI2cTR0d1xuZERLSURxTE84ZXhxS2dSaWY4RHBDYWZFNGROU0VMOVE4b1U5aExuWkVDbHpUY2R0a2pWY3J4cHRvMzdQVko1MlxuUjZFQW5CZWhBZ01CQUFFQ2dnRUFZU3dvMFpMUTl1WWJqbTVtamIwVWFoaTFlRzlpMnZuS09BeEdtQTdiNWhCaVxuL0VjNVhRbUdtOWdCUEtQZFZZZHk4dG5rSktmOXA5V2RWSE1SOGVDdCtTRFViY2hhbGFMbnZFKytHc29BOXE5RlxuUUpSU0xPTmpVbC9haHVnK1BDNnNPNXVpR2NHQU14MGhzZTUvMEVnbW45czhna0N5QlYxRjBJaDVBaVJsNXNMQlxuaUJ1YVlWSlVmc1p6NFhmOFNtU1ZsTHNyaHVQZkVBeU9IUHVkZUYrUW93UWoxR280TlpodDJ6dm9idVJtTmpqNlxuTVBtaGpHbVNOaWxLMXBqRmd2WjdJUHhJTlArZHNNTE5JS2IxQ2NDeTNLS3hiemJ3MDlIcm01cysrNzhXOGpWV1xuNjd4bDJjRFFGZVZONkx4enM4d2RQNlJiclpnSG50OFN6MDcwRU9XQ1V3S0JnUUQ4eG1nekl0dkRLYkhlNVpydFxuZ3BzcEVHa1V6SmsvMU0vRUdQdFgzaW1YbkxyRUhYbE9rWnlvWGxYTUp2YTFSWnp2TEFoeWgzVGViMzlOL2thV1xuLzRIcXZnN3loaUFjbnArNUlQdUJTbEplK3NIenRhOEFVbDNpTEJYRWJ5bGRZdFkwMzFLZWpwdUMzWUtRc2F6YlxuV1JYcXowcHVMOVRlL2Nia3lscVhyYmU5SXdLQmdRRFMrZDh0ajRqZ0F1Y1FzQm5uRjFJbzdLOHp3Qzl2M2RmQ1xudnNsWTRTWXh2ZEhJbGpvOFpUTmY3Y05xYVZjZGg4THdnUVJZbi9SSE14cUxrTUo1Q08vM2VGaVZydUdkNmg1d1xuSkRPcjZkbW5JVklWTDZCdlpEa0pueDhJNVpscnB4MzFma3ZPekpFRENtV29lNGpUaG0zV1VFeVpUTFlWRFdzWFxudlJwb2hIbHVhd0tCZ0VVOGwwZ0NjVTJJdXliQm4ya1ZFQ2owVE1RY3NwRlFXa1J0VDFNbkVCOXVGNTRtTUpiN1xudlh4RXNwMkR3cW11VXFrVVY0Ly9XRnloRDY2dVNtbUx2T3N1ZWV1bUgxK1hkMHAvSlVTcHRkdzhOU25yQnU5QVxub0dTV0RMUk1lbmtRM0htSS9obGVHR3lFL2dGaUdXWFBoZmhXSlIzL1RnQnlaS3RBWGdZVDJETWZBb0dBWk4wWlxuR2NzWmdSOWlJTlJRVGU4VVZJUnpicVpmQjNoa0FyTDd5QVk4SUdQRHU4WTJxVkVvc3FBVllQWmpzN2FJT0RzMlxuUExpY0xMMzkzdU9pVmdNejFuZ3V3Y0VPRkZVdG9DZHVuSzM4Wks3RmMyT0ZyRHVhR1VOOXJ0ODE3Z1hEaU82TVxuaDUyOVpscStKMEtJTTdoOUlvelpVaUVlbkFvQ1BTTW5VUGlrcFdrQ2dZRUFxcXlnN09VYzM0K1ErOEFtaHVjK1xuVmxPMVJZTWM4Ymdsdk1uNGZ0Z0htTXhURy9wbzFjN3RqVGpUL1docXB4c1pCWWVEMmxxT0ltT3JSa0UzNzc1NFxuVThnZkdEakNpelVGUlF3dEdqQkIvUk9QU1hBTUEvT01sbm1YSmhTNTVsYWwvSHF2RGdEMWRlS3crbkVyVkxIRlxuS21ydTR2UGVOOGxwVmdYVU9YOVN1OTQ9XG4tLS0tLUVORCBQUklWQVRFIEtFWS0tLS0tXG4iLCAKCQkJImNsaWVudF9lbWFpbCI6ICJjYXNlLXN0dWR5LWJ1Y2tldEBxdWVyZW50LTEuaWFtLmdzZXJ2aWNlYWNjb3VudC5jb20iLCAKCQkJImNsaWVudF9pZCI6ICIxMTU0OTE3NzQ0NTU4MTUxNTE3MTUiLCAKCQkJImF1dGhfdXJpIjogImh0dHBzOi8vYWNjb3VudHMuZ29vZ2xlLmNvbS9vL29hdXRoMi9hdXRoIiwgCgkJCSJ0b2tlbl91cmkiOiAiaHR0cHM6Ly9vYXV0aDIuZ29vZ2xlYXBpcy5jb20vdG9rZW4iLCAKCQkJImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9vYXV0aDIvdjEvY2VydHMiLCAKCQkJImNsaWVudF94NTA5X2NlcnRfdXJsIjogImh0dHBzOi8vd3d3Lmdvb2dsZWFwaXMuY29tL3JvYm90L3YxL21ldGFkYXRhL3g1MDkvY2FzZS1zdHVkeS1idWNrZXQlNDBxdWVyZW50LTEuaWFtLmdzZXJ2aWNlYWNjb3VudC5jb20iLCAKCQkJInVuaXZlcnNlX2RvbWFpbiI6ICJnb29nbGVhcGlzLmNvbSJ9".to_string(),
        };


		// Initialize the GCS storage
		match get_gcs_storage(gcs_config) {
			Ok(storage) => {
				// Define the path of the file to retrieve
				let path = Path::new("");

				assert!(storage.check_connectivity().await.is_ok(), "Failed to connect");

				let files_list = storage.list_files(path).await;
				assert!(files_list.is_ok(), "Failed to list files");

				// Perform the get_all operation
				match files_list {
					Ok(files) => {
						for file in files {
							let temp_path = Path::new(&file);
							let result = storage.get_all(temp_path).await;

							// Check if the result is as expected
							assert!(result.is_ok());

						}
					}
					Err(e) => {
						println!("Failed to initialize GCS storage: {:?}", e);
            			assert!(false, "Storage initialization failed with error: {:?}", e);
					}
				}
				
				
			},
			Err(e) => {
				println!("Failed to initialize GCS storage: {:?}", e);
				assert!(false, "Storage initialization failed with error: {:?}", e);
			},
		}
	}
}
