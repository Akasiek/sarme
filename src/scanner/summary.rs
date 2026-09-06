#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ScanSummary {
    scan_id: i64,
    discovered: i64,
    processed: i64,
    new: i64,
    updated: i64,
    unchanged: i64,
    removed: i64,
    errors: i64,
}

impl ScanSummary {
    pub(super) const fn new(scan_id: i64, discovered: i64) -> Self {
        Self {
            scan_id,
            discovered,
            processed: 0,
            new: 0,
            updated: 0,
            unchanged: 0,
            removed: 0,
            errors: 0,
        }
    }

    pub(super) fn record_new(&mut self) {
        self.new += 1;
    }

    pub(super) fn record_updated(&mut self) {
        self.updated += 1;
    }

    pub(super) fn record_unchanged(&mut self) {
        self.unchanged += 1;
    }

    pub(super) fn finish(&mut self, removed: i64, errors: i64) {
        self.processed = self.new + self.updated;
        self.removed = removed;
        self.errors = errors;
    }

    pub(crate) const fn scan_id(self) -> i64 {
        self.scan_id
    }

    pub(crate) const fn discovered(self) -> i64 {
        self.discovered
    }

    pub(crate) const fn processed(self) -> i64 {
        self.processed
    }

    pub(crate) const fn new_tracks(self) -> i64 {
        self.new
    }

    pub(crate) const fn updated_tracks(self) -> i64 {
        self.updated
    }

    pub(crate) const fn unchanged_tracks(self) -> i64 {
        self.unchanged
    }

    pub(crate) const fn removed_tracks(self) -> i64 {
        self.removed
    }

    pub(crate) const fn errors(self) -> i64 {
        self.errors
    }
}
