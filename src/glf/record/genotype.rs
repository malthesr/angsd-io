/// A diploid, diallelic genotype used for indexing a [`Record`](crate::glf::Record).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Genotype {
    /// AA genotype.
    AA = 0,
    /// AC genotype.
    AC = 1,
    /// AG genotype.
    AG = 2,
    /// AT genotype.
    AT = 3,
    /// CC genotype.
    CC = 4,
    /// CG genotype.
    CG = 5,
    /// CT genotype.
    CT = 6,
    /// GG genotype.
    GG = 7,
    /// GT genotype.
    GT = 8,
    /// TT genotype.
    TT = 9,
}
