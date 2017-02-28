use core::SegmentId;


#[derive(Clone, Debug, RustcDecodable,RustcEncodable)]
struct DeleteMeta {
    num_deleted_docs: u32,
    opstamp: u64,
}

/// SegmentMeta contains simple meta information about a segment.
///
/// For instance the number of docs it contains,
/// how many are deleted, etc.
#[derive(Clone, Debug, RustcDecodable,RustcEncodable)]
pub struct SegmentMeta {
    segment_id: SegmentId,
    max_doc: u32,
    deletes: Option<DeleteMeta>, 
}

impl SegmentMeta {

    /// Creates a new segment meta for 
    /// a segment with no deletes and no documents.
    pub fn new(segment_id: SegmentId) -> SegmentMeta {
        SegmentMeta {
            segment_id: segment_id,
            max_doc: 0,
            deletes: None,
        }
    }

    /// Returns the segment id.
    pub fn id(&self) -> SegmentId {
        self.segment_id
    }

    /// Returns the number of deleted documents.
    pub fn num_deleted_docs(&self) -> u32 {
        self.deletes
            .as_ref()
            .map(|delete_meta| delete_meta.num_deleted_docs)
            .unwrap_or(0u32)
    }

    /// Return the highest doc id + 1
    ///
    /// If there are no deletes, then num_docs = max_docs
    /// and all the doc ids contains in this segment
    /// are exactly (0..max_doc).
    pub fn max_doc(&self) -> u32 {
        self.max_doc
    }

    /// Return the number of documents in the segment.
    pub fn num_docs(&self) -> u32 {
        self.max_doc() - self.num_deleted_docs()
    }

    /// Returns the opstamp of the last delete operation
    /// taken in account in this segment.
    pub fn delete_opstamp(&self) -> Option<u64> {
        self.deletes
            .as_ref()
            .map(|delete_meta| delete_meta.opstamp)
    }

    /// Returns true iff the segment meta contains
    /// delete information.
    pub fn has_deletes(&self) -> bool {
        self.deletes.is_some()
    }

    #[doc(hidden)]
    pub fn set_max_doc(&mut self, max_doc: u32) {
        self.max_doc = max_doc;
    }

    #[doc(hidden)]
    pub fn set_delete_meta(&mut self, num_deleted_docs: u32, opstamp: u64) {
        self.deletes = Some(DeleteMeta {
            num_deleted_docs: num_deleted_docs,
            opstamp: opstamp,
        });
    }
}