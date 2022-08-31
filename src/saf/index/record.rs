use std::{fmt, marker::PhantomData};

use crate::saf::{Version, V3};

/// A SAF index record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record<V: Version = V3> {
    name: String,
    sites: usize,
    position_offset: u64,
    value_offset: u64,
    v: PhantomData<V>,
}

impl<V> Record<V>
where
    V: Version,
{
    /// Returns the reference sequence name, consuming `self`.
    pub fn into_name(self) -> String {
        self.name
    }

    /// Returns the reference sequence name.
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Returns a mutable reference to the reference sequence name.
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    /// Creates a new record.
    pub fn new(name: String, sites: usize, position_offset: u64, value_offset: u64) -> Self {
        Self {
            name,
            sites,
            position_offset,
            value_offset,
            v: PhantomData,
        }
    }

    /// Returns the position offset.
    ///
    /// This is the byte offset into the position file at which the reference sequence data begins.
    pub fn position_offset(&self) -> u64 {
        self.position_offset
    }

    /// Returns a mutable reference to the position offset.
    ///
    /// This is the byte offset into the position file at which the reference sequence data begins.
    pub fn position_offset_mut(&mut self) -> &mut u64 {
        &mut self.position_offset
    }

    /// Returns the record sites.
    ///
    /// This is the number of sites on the reference sequence contained in the position and value
    /// files.
    pub fn sites(&self) -> usize {
        self.sites
    }

    /// Returns a mutable reference to the record sites.
    ///
    /// This is the number of sites on the reference sequence contained in the position and value
    /// files.
    pub fn sites_mut(&mut self) -> &mut usize {
        &mut self.sites
    }

    /// Returns the value offset.
    ///
    /// This is the byte offset into the value file at which the reference sequence data begins.
    pub fn value_offset(&self) -> u64 {
        self.value_offset
    }

    /// Returns a mutable reference to the value offset.
    ///
    /// This is the byte offset into the value file at which the reference sequence data begins.
    pub fn value_offset_mut(&mut self) -> &mut u64 {
        &mut self.value_offset
    }
}

impl<V> fmt::Display for Record<V>
where
    V: Version,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "#contig=<ID={}, sites={}>", self.name, self.sites)
    }
}
