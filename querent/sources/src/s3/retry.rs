use std::fmt::Debug;

use aws_runtime::retries::classifiers::{THROTTLING_ERRORS, TRANSIENT_ERRORS};
use aws_sdk_s3::{
	error::SdkError,
	operation::{
		abort_multipart_upload::AbortMultipartUploadError,
		complete_multipart_upload::CompleteMultipartUploadError,
		create_multipart_upload::CreateMultipartUploadError, delete_object::DeleteObjectError,
		delete_objects::DeleteObjectsError, get_object::GetObjectError,
		head_object::HeadObjectError, list_objects_v2::ListObjectsV2Error,
		put_object::PutObjectError, upload_part::UploadPartError,
	},
};
use common::retry::{retry_with_mockable_sleep, Retry, RetryParams, Retryable, TokioSleep};
use futures::{Future, TryFutureExt};

pub trait AwsRetryable {
	fn is_retryable(&self) -> bool {
		false
	}
}

impl<E> AwsRetryable for Retry<E> {
	fn is_retryable(&self) -> bool {
		match self {
			Retry::Transient(_) => true,
			Retry::Permanent(_) => false,
		}
	}
}

#[derive(Debug)]
struct AwsRetryableWrapper<E>(E);

impl<E> Retryable for AwsRetryableWrapper<E>
where
	E: AwsRetryable,
{
	fn is_retryable(&self) -> bool {
		self.0.is_retryable()
	}
}

pub async fn aws_retry<U, E, Fut>(retry_params: &RetryParams, f: impl Fn() -> Fut) -> Result<U, E>
where
	Fut: Future<Output = Result<U, E>>,
	E: AwsRetryable + Debug + 'static,
{
	retry_with_mockable_sleep(retry_params, || f().map_err(AwsRetryableWrapper), TokioSleep)
		.await
		.map_err(|error| error.0)
}

impl<E> AwsRetryable for SdkError<E>
where
	E: AwsRetryable,
{
	fn is_retryable(&self) -> bool {
		match self {
			SdkError::ConstructionFailure(_) => false,
			SdkError::TimeoutError(_) => true,
			SdkError::DispatchFailure(_) => false,
			SdkError::ResponseError(_) => true,
			SdkError::ServiceError(error) => error.err().is_retryable(),
			_ => false,
		}
	}
}

fn is_retryable(meta: &aws_sdk_s3::error::ErrorMetadata) -> bool {
	if let Some(code) = meta.code() {
		THROTTLING_ERRORS.contains(&code) || TRANSIENT_ERRORS.contains(&code)
	} else {
		false
	}
}

impl AwsRetryable for GetObjectError {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}

impl AwsRetryable for DeleteObjectError {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}

impl AwsRetryable for DeleteObjectsError {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}

impl AwsRetryable for UploadPartError {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}

impl AwsRetryable for CompleteMultipartUploadError {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}

impl AwsRetryable for AbortMultipartUploadError {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}

impl AwsRetryable for CreateMultipartUploadError {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}

impl AwsRetryable for PutObjectError {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}

impl AwsRetryable for HeadObjectError {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}

impl AwsRetryable for ListObjectsV2Error {
	fn is_retryable(&self) -> bool {
		is_retryable(self.meta())
	}
}
