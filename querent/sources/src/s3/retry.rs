// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.xyz).

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
