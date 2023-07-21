use crate::bucket::Bucket;

#[derive(Debug)]
#[non_exhaustive]
pub enum Event {
    BucketAddStarted(String),
    BucketAddFailed(String),
    BucketAddFinished(String),
    BucketListItem(Bucket),
    BucketUpdateStarted(String),
    BucketUpdateFailed(BucketUpdateFailedCtx),
    BucketUpdateSuccessed(String),
    BucketUpdateFinished,
    SessionTerminated,
}

#[derive(Debug)]
pub struct BucketUpdateFailedCtx {
    pub name: String,
    pub err_msg: String,
}
