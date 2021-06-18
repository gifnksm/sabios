use super::{BiosParameterBlock, FatEntry};

#[derive(Debug, Clone)]
pub(crate) struct ClusterChain<'a> {
    bpb: &'a dyn BiosParameterBlock,
    next_entry: Option<FatEntry>,
}

impl<'a> ClusterChain<'a> {
    pub(super) fn new(bpb: &'a dyn BiosParameterBlock, cluster: u32) -> Self {
        Self {
            bpb,
            next_entry: Some(FatEntry::Used(cluster)),
        }
    }

    pub(super) fn bpb(&'a self) -> &'a dyn BiosParameterBlock {
        self.bpb
    }
}

impl Iterator for ClusterChain<'_> {
    type Item = Result<u32, FatEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_entry = self.next_entry?;
        let res = match next_entry {
            FatEntry::Used(next_cluster) => {
                self.next_entry = Some(self.bpb.fat_entry(next_cluster));
                Ok(next_cluster)
            }
            FatEntry::UsedEof(next_cluster) => {
                self.next_entry = None;
                Ok(next_cluster)
            }
            FatEntry::Unused | FatEntry::Reserved | FatEntry::Bad => {
                self.next_entry = None;
                Err(next_entry)
            }
        };
        Some(res)
    }
}
