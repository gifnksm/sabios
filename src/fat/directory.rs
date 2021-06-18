use super::{ClusterChain, DirectoryEntry, FatEntry};
use crate::fat::FileAttribute;
use core::{mem, slice};

#[derive(Debug)]
pub(crate) enum Directory<'a> {
    RootDir(&'a [DirectoryEntry]),
    ClusterChain(ClusterChain<'a>),
}

impl<'a> Directory<'a> {
    pub(super) fn new_root_dir(sectors: &'a [DirectoryEntry]) -> Self {
        Self::RootDir(sectors)
    }

    pub(super) fn new_cluster_chain(chain: ClusterChain<'a>) -> Self {
        Self::ClusterChain(chain)
    }

    pub(crate) fn entries(&self) -> DirectoryEntries<'a> {
        match self {
            Directory::RootDir(entries) => DirectoryEntries {
                iter: entries.iter(),
                chain: None,
            },
            Directory::ClusterChain(chain) => DirectoryEntries {
                iter: [].iter(),
                chain: Some(chain.clone()),
            },
        }
    }
}

#[derive(Debug)]
pub(crate) struct DirectoryEntries<'a> {
    iter: slice::Iter<'a, DirectoryEntry>,
    chain: Option<ClusterChain<'a>>,
}

impl<'a> Iterator for DirectoryEntries<'a> {
    type Item = Result<&'a DirectoryEntry, FatEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            for entry in &mut self.iter {
                if entry.name()[0] == 0x00 {
                    // stop iteration
                    self.iter = [].iter();
                    return None;
                }
                if entry.name()[0] == 0xe5 {
                    continue;
                }
                if entry.attr() == FileAttribute::LFN {
                    continue;
                }
                return Some(Ok(entry));
            }
            let chain = self.chain.as_mut()?;
            let cluster = match chain.next()? {
                Ok(cluster) => cluster,
                Err(err) => return Some(Err(err)),
            };
            let bpb = chain.bpb();
            let sectors_per_cluster = usize::from(bpb.sectors_per_cluster());
            let bytes_per_sector = usize::from(bpb.bytes_per_sector());
            let entry_size = mem::size_of::<DirectoryEntry>();

            let sector = bpb.cluster_sector(cluster);
            let data = bpb.sector_ptr(sector).cast();
            let len = (sectors_per_cluster * bytes_per_sector + entry_size - 1) / entry_size;

            self.iter = unsafe { slice::from_raw_parts(data, len) }.iter();
        }
    }
}
