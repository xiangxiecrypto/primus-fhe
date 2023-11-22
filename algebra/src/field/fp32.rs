use std::collections::{HashMap, HashSet};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::sync::{Arc, Mutex};

use num_traits::{Inv, One, Pow, Zero};
use once_cell::sync::{Lazy, OnceCell};
use rand::distributions::uniform::{SampleUniform, UniformInt, UniformSampler};
use rand::distributions::{Distribution, Standard, Uniform};
use rand::{thread_rng, Rng};
use rand_distr::{Bernoulli, Normal, WeightedIndex};

use crate::ring::Ring;
use crate::AlgebraError;
use crate::{
    field::{prime_fields::MulFactor, Field, FieldDistribution, NTTField, PrimeField},
    modulo_traits::{
        AddModulo, AddModuloAssign, DivModulo, DivModuloAssign, InvModulo, MulModulo,
        MulModuloAssign, NegModulo, PowModulo, SubModulo, SubModuloAssign,
    },
    modulus::{Modulus, MulModuloFactor},
    transformation::NTTTable,
    utils::{Prime, ReverseLsbs},
};

const P: u32 = 0x7e00001;

/// A finite Field type, whose inner size is 32bits.
///
/// Now, it's focused on the prime field.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct Fp32(u32);

impl std::fmt::Display for Fp32 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[({})_{}]", self.0, P)
    }
}

/// A helper trait to get the modulus of the field.
pub trait BarrettConfig<const P: u32> {
    /// The modulus of the field.
    const BARRETT_MODULUS: Modulus<u32>;

    /// Get the barrett modulus of the field.
    #[inline]
    fn barrett_modulus() -> Modulus<u32> {
        Self::BARRETT_MODULUS
    }
}

impl BarrettConfig<P> for Fp32 {
    const BARRETT_MODULUS: Modulus<u32> = Modulus::<u32>::new(P);
}

impl Fp32 {
    /// Creates a new [`Fp32`].
    #[inline]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return inner value
    #[inline]
    pub fn inner(self) -> u32 {
        self.0
    }
}

impl From<u32> for Fp32 {
    /// Converts an unsigned 32-bit integer into a [`Fp32`] value.
    ///
    /// # Arguments
    ///
    /// * `value` - The unsigned 32-bit integer to convert.
    ///
    /// # Returns
    ///
    /// The converted [`Fp32`] value.
    #[inline]
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Zero for Fp32 {
    #[inline]
    fn zero() -> Self {
        Self(0)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        0 == self.0
    }
}

impl One for Fp32 {
    #[inline]
    fn one() -> Self {
        Self(1)
    }
}

static STANDARD_FP32: Lazy<Uniform<Fp32>> =
    Lazy::new(|| Uniform::new_inclusive(Fp32(0), Fp32(P - 1)));

impl Distribution<Fp32> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp32 {
        STANDARD_FP32.sample(rng)
    }
}

/// The binary distribution for [`Fp32`].
///
/// prob\[1] = prob\[0] = 0.5
#[derive(Clone, Copy, Debug)]
pub struct BinaryFp32 {
    inner: Bernoulli,
}

impl BinaryFp32 {
    /// Creates a new [`BinaryFp32`].
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: Bernoulli::new(0.5).unwrap(),
        }
    }
}

impl Default for BinaryFp32 {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Distribution<Fp32> for BinaryFp32 {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp32 {
        if self.inner.sample(rng) {
            Fp32(1)
        } else {
            Fp32(0)
        }
    }
}

/// The ternary distribution for [`Fp32`].
///
/// prob\[1] = prob\[-1] = 0.25
///
/// prob\[0] = 0.5
#[derive(Clone, Debug)]
pub struct TernaryFp32 {
    inner: WeightedIndex<usize>,
}

impl TernaryFp32 {
    /// Creates a new [`TernaryFp32`].
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: WeightedIndex::new([1, 2, 1]).unwrap(),
        }
    }
}

impl Default for TernaryFp32 {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Distribution<Fp32> for TernaryFp32 {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp32 {
        const VALUES: [Fp32; 3] = [Fp32(P - 1), Fp32(0), Fp32(1)];
        VALUES[self.inner.sample(rng)]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UniformFp32(UniformInt<u32>);

impl UniformSampler for UniformFp32 {
    type X = Fp32;

    #[inline]
    fn new<B1, B2>(low: B1, high: B2) -> Self
    where
        B1: rand::distributions::uniform::SampleBorrow<Self::X> + Sized,
        B2: rand::distributions::uniform::SampleBorrow<Self::X> + Sized,
    {
        UniformFp32(UniformInt::<u32>::new_inclusive(
            low.borrow().0,
            high.borrow().0 - 1,
        ))
    }

    #[inline]
    fn new_inclusive<B1, B2>(low: B1, high: B2) -> Self
    where
        B1: rand::distributions::uniform::SampleBorrow<Self::X> + Sized,
        B2: rand::distributions::uniform::SampleBorrow<Self::X> + Sized,
    {
        let high = if high.borrow().0 >= P - 1 {
            P - 1
        } else {
            high.borrow().0
        };
        UniformFp32(UniformInt::<u32>::new_inclusive(low.borrow().0, high))
    }

    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::X {
        Fp32(self.0.sample(rng))
    }
}

impl SampleUniform for Fp32 {
    type Sampler = UniformFp32;
}

/// The normal distribution `N(mean, std_dev**2)` for [`Fp32`].
#[derive(Clone, Copy, Debug)]
pub struct NormalFp32 {
    inner: Normal<f64>,
}

impl NormalFp32 {
    /// Construct, from mean and standard deviation
    ///
    /// Parameters:
    ///
    /// -   mean (`μ`, unrestricted)
    /// -   standard deviation (`σ`, must be finite)
    #[inline]
    pub fn new(mean: f64, std_dev: f64) -> Result<NormalFp32, AlgebraError> {
        match Normal::new(mean, std_dev) {
            Ok(inner) => Ok(NormalFp32 { inner }),
            Err(_) => Err(AlgebraError::DistributionError),
        }
    }

    /// Returns the mean (`μ`) of the distribution.
    #[inline]
    pub fn mean(&self) -> f64 {
        self.inner.mean()
    }

    /// Returns the standard deviation (`σ`) of the distribution.
    #[inline]
    pub fn std_dev(&self) -> f64 {
        self.inner.std_dev()
    }
}

impl Distribution<Fp32> for NormalFp32 {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp32 {
        const FLOAT_P: f64 = P as f64;
        let mut value = self.inner.sample(rng);
        while value < 0. {
            value += FLOAT_P;
        }
        while value >= FLOAT_P {
            value -= FLOAT_P;
        }
        Fp32(value as u32)
    }
}

impl FieldDistribution for Fp32 {
    type BinaryDistribution = BinaryFp32;

    type TernaryDistribution = TernaryFp32;

    type NormalDistribution = NormalFp32;

    #[inline]
    fn binary_distribution() -> Self::BinaryDistribution {
        BinaryFp32::new()
    }

    #[inline]
    fn ternary_distribution() -> Self::TernaryDistribution {
        TernaryFp32::new()
    }

    #[inline]
    fn normal_distribution(
        mean: f64,
        std_dev: f64,
    ) -> Result<Self::NormalDistribution, AlgebraError> {
        NormalFp32::new(mean, std_dev)
    }
}

impl Add<Self> for Fp32 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.add_reduce(rhs.0, P))
    }
}

impl Add<&Self> for Fp32 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: &Self) -> Self::Output {
        Self(self.0.add_reduce(rhs.0, P))
    }
}

impl AddAssign<Self> for Fp32 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0.add_reduce_assign(rhs.0, P)
    }
}

impl AddAssign<&Self> for Fp32 {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        self.0.add_reduce_assign(rhs.0, P)
    }
}

impl Sub<Self> for Fp32 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.sub_reduce(rhs.0, P))
    }
}

impl Sub<&Self> for Fp32 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: &Self) -> Self::Output {
        Self(self.0.sub_reduce(rhs.0, P))
    }
}

impl SubAssign<Self> for Fp32 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0.sub_reduce_assign(rhs.0, P)
    }
}

impl SubAssign<&Self> for Fp32 {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) {
        self.0.sub_reduce_assign(rhs.0, P)
    }
}

impl Mul<Self> for Fp32 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.mul_reduce(rhs.0, &Self::BARRETT_MODULUS))
    }
}

impl Mul<&Self> for Fp32 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: &Self) -> Self::Output {
        Self(self.0.mul_reduce(rhs.0, &Self::BARRETT_MODULUS))
    }
}

impl MulAssign<Self> for Fp32 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.0.mul_reduce_assign(rhs.0, &Self::BARRETT_MODULUS)
    }
}

impl MulAssign<&Self> for Fp32 {
    #[inline]
    fn mul_assign(&mut self, rhs: &Self) {
        self.0.mul_reduce_assign(rhs.0, &Self::BARRETT_MODULUS)
    }
}

impl Div<Self> for Fp32 {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0.div_reduce(rhs.0, &Self::BARRETT_MODULUS))
    }
}

impl Div<&Self> for Fp32 {
    type Output = Self;

    #[inline]
    fn div(self, rhs: &Self) -> Self::Output {
        Self(self.0.div_reduce(rhs.0, &Self::BARRETT_MODULUS))
    }
}

impl DivAssign<Self> for Fp32 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.0.div_reduce_assign(rhs.0, &Self::BARRETT_MODULUS);
    }
}

impl DivAssign<&Self> for Fp32 {
    #[inline]
    fn div_assign(&mut self, rhs: &Self) {
        self.0.div_reduce_assign(rhs.0, &Self::BARRETT_MODULUS);
    }
}

impl Neg for Fp32 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self(self.0.neg_reduce(P))
    }
}

impl Inv for Fp32 {
    type Output = Self;

    #[inline]
    fn inv(self) -> Self::Output {
        Self(self.0.inv_reduce(P))
    }
}

impl Pow<<Self as Ring>::Order> for Fp32 {
    type Output = Self;

    #[inline]
    fn pow(self, rhs: <Self as Ring>::Order) -> Self::Output {
        Self(self.0.pow_reduce(rhs, &Self::BARRETT_MODULUS))
    }
}

impl Ring for Fp32 {
    type Scalar = u32;

    type Order = u32;

    type Base = u32;

    #[inline]
    fn order() -> Self::Order {
        P
    }

    #[inline]
    fn mul_scalar(&self, scalar: Self::Scalar) -> Self {
        Self(self.0.mul_reduce(scalar, &Self::BARRETT_MODULUS))
    }
}

impl Field for Fp32 {
    type Modulus = u32;

    #[inline]
    fn modulus() -> Self::Modulus {
        P
    }
}

impl PrimeField for Fp32 {
    /// Check [`Self`] is a prime field.
    #[inline]
    fn is_prime_field() -> bool {
        <Self as BarrettConfig<P>>::BARRETT_MODULUS.probably_prime(20)
    }
}

static mut NTT_TABLE: OnceCell<HashMap<u32, Arc<NTTTable<Fp32>>>> = OnceCell::new();
static NTT_MUTEX: Mutex<()> = Mutex::new(());

impl NTTField for Fp32 {
    type Table = NTTTable<Self>;

    type Root = MulFactor<Self>;

    type Degree = u32;

    #[inline]
    fn decompose_len(basis: Self::Base) -> usize {
        const fn div_ceil(lhs: u32, rhs: u32) -> u32 {
            let d = lhs / rhs;
            let r = lhs % rhs;
            if r > 0 {
                d + 1
            } else {
                d
            }
        }
        debug_assert!(basis.is_power_of_two());
        div_ceil(Self::barrett_modulus().bit_count(), basis.trailing_zeros()) as usize
    }

    fn decompose(&self, basis: Self::Base) -> Vec<Self> {
        let mut temp = self.0;
        let bits = basis.trailing_zeros();

        let len = Self::decompose_len(basis);
        let mask = u32::MAX >> (u32::BITS - bits);
        let mut ret: Vec<Self> = Vec::with_capacity(len);

        while !temp.is_zero() {
            ret.push(Self(temp & mask));
            temp >>= bits;
        }

        ret.resize(len, Fp32(0));

        ret
    }

    #[inline]
    fn from_root(root: Self::Root) -> Self {
        root.value()
    }

    #[inline]
    fn to_root(&self) -> Self::Root {
        Self::Root::new(*self, Fp32((((self.0 as u64) << 32) / P as u64) as u32))
    }

    #[inline]
    fn mul_root(&self, root: Self::Root) -> Self {
        let r = MulModuloFactor::<u32> {
            value: root.value().0,
            quotient: root.quotient().0,
        };

        Self(self.0.mul_reduce(r, P))
    }

    #[inline]
    fn mul_root_assign(&mut self, root: Self::Root) {
        let r = MulModuloFactor::<u32> {
            value: root.value().0,
            quotient: root.quotient().0,
        };

        self.0.mul_reduce_assign(r, P);
    }

    #[inline]
    fn is_primitive_root(root: Self, degree: Self::Degree) -> bool {
        debug_assert!(root.0 < P);
        assert!(
            degree > 1 && degree.is_power_of_two(),
            "degree must be a power of two and bigger than 1"
        );

        if root.is_zero() {
            return false;
        }

        root.pow(degree >> 1).0 == P - 1
    }

    fn try_primitive_root(degree: Self::Degree) -> Result<Self, crate::AlgebraError> {
        // p-1
        let modulus_sub_one = P - 1;

        // (p-1)/n
        let quotient = modulus_sub_one / degree;

        // (p-1) must be divisible by n
        if modulus_sub_one != quotient * degree {
            return Err(crate::AlgebraError::NoPrimitiveRoot {
                degree: degree.to_string(),
                modulus: P.to_string(),
            });
        }

        let mut rng = thread_rng();
        let distr = rand::distributions::Uniform::new_inclusive(Self(2), Self(P - 1));

        let mut w = Zero::zero();

        if (0..100).any(|_| {
            w = rng.sample(distr).pow(quotient);
            Self::is_primitive_root(w, degree)
        }) {
            Ok(w)
        } else {
            Err(crate::AlgebraError::NoPrimitiveRoot {
                degree: degree.to_string(),
                modulus: P.to_string(),
            })
        }
    }

    fn try_minimal_primitive_root(degree: Self::Degree) -> Result<Self, crate::AlgebraError> {
        let mut root = Self::try_primitive_root(degree)?;

        let generator_sq = root.square();
        let mut current_generator = root;

        for _ in 0..degree {
            if current_generator < root {
                root = current_generator;
            }

            current_generator *= generator_sq;
        }

        Ok(root)
    }

    fn generate_ntt_table(log_n: u32) -> Result<NTTTable<Self>, crate::AlgebraError> {
        let n = 1usize << log_n;

        let root = Self::try_minimal_primitive_root((n * 2).try_into().unwrap())?;
        let inv_root = root.inv();

        let root_factor = root.to_root();
        let mut power = root;

        let mut root_powers = vec![<Self as NTTField>::Root::default(); n];
        root_powers[0] = Self::one().to_root();
        for i in 1..n {
            root_powers[i.reverse_lsbs(log_n)] = power.to_root();
            power.mul_root_assign(root_factor);
        }

        let inv_root_factor = inv_root.to_root();
        let mut inv_root_powers = vec![<Self as NTTField>::Root::default(); n];
        power = inv_root;

        inv_root_powers[0] = Self::one().to_root();
        for i in 1..n {
            inv_root_powers[(i - 1).reverse_lsbs(log_n) + 1] = power.to_root();
            power.mul_root_assign(inv_root_factor);
        }
        let inv_degree = Self(n as u32).inv().to_root();

        Ok(NTTTable::new(
            root,
            inv_root,
            log_n,
            n,
            inv_degree,
            root_powers,
            inv_root_powers,
        ))
    }

    fn get_ntt_table(log_n: u32) -> Result<Arc<Self::Table>, crate::AlgebraError> {
        if let Some(tables) = unsafe { NTT_TABLE.get() } {
            if let Some(t) = tables.get(&log_n) {
                return Ok(Arc::clone(t));
            }
        }

        Self::init_ntt_table(&[log_n])?;
        Ok(Arc::clone(unsafe {
            NTT_TABLE.get().unwrap().get(&log_n).unwrap()
        }))
    }

    fn init_ntt_table(log_ns: &[u32]) -> Result<(), crate::AlgebraError> {
        let _g = NTT_MUTEX.lock().unwrap();
        match unsafe { NTT_TABLE.get_mut() } {
            Some(tables) => {
                let new_log_ns: HashSet<u32> = log_ns.iter().copied().collect();
                let old_log_ns: HashSet<u32> = tables.keys().copied().collect();
                let difference = new_log_ns.difference(&old_log_ns);

                for &log_n in difference {
                    let temp_table = Self::generate_ntt_table(log_n)?;
                    tables.insert(log_n, Arc::new(temp_table));
                }

                Ok(())
            }
            None => {
                let log_ns: HashSet<u32> = log_ns.iter().copied().collect();
                let mut map = HashMap::with_capacity(log_ns.len());

                for log_n in log_ns {
                    let temp_table = Self::generate_ntt_table(log_n)?;
                    map.insert(log_n, Arc::new(temp_table));
                }

                if unsafe { NTT_TABLE.set(map).is_err() } {
                    Err(crate::AlgebraError::NTTTableError)
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modulo_traits::PowModulo;
    use rand::thread_rng;

    #[test]
    fn test_fp() {
        const P: u32 = Fp32::BARRETT_MODULUS.value();

        let distr = rand::distributions::Uniform::new(0, P);
        let mut rng = thread_rng();

        type FF = Fp32;
        assert!(FF::is_prime_field());

        // add
        let a = rng.sample(distr);
        let b = rng.sample(distr);
        let c = (a + b) % P;
        assert_eq!(FF::from(a) + FF::from(b), FF::from(c));

        // add_assign
        let mut a = FF::from(a);
        a += FF::from(b);
        assert_eq!(a, FF::from(c));

        // sub
        let a = rng.sample(distr);
        let b = rng.gen_range(0..=a);
        let c = (a - b) % P;
        assert_eq!(FF::from(a) - FF::from(b), FF::from(c));

        // sub_assign
        let mut a = FF::from(a);
        a -= FF::from(b);
        assert_eq!(a, FF::from(c));

        // mul
        let a = rng.sample(distr);
        let b = rng.sample(distr);
        let c = ((a as u64 * b as u64) % P as u64) as u32;
        assert_eq!(FF::from(a) * FF::from(b), FF::from(c));

        // mul_assign
        let mut a = FF::from(a);
        a *= FF::from(b);
        assert_eq!(a, FF::from(c));

        // div
        let a = rng.sample(distr);
        let b = rng.sample(distr);
        let b_inv = b.pow_reduce(P - 2, &Modulus::<u32>::new(P));
        let c = ((a as u64 * b_inv as u64) % P as u64) as u32;
        assert_eq!(FF::from(a) / FF::from(b), FF::from(c));

        // div_assign
        let mut a = FF::from(a);
        a /= FF::from(b);
        assert_eq!(a, FF::from(c));

        // neg
        let a = rng.sample(distr);
        let a_neg = -FF::from(a);
        assert_eq!(FF::from(a) + a_neg, Zero::zero());

        // inv
        let a = rng.sample(distr);
        let a_inv = a.pow_reduce(P - 2, &Modulus::<u32>::new(P));
        assert_eq!(FF::from(a).inv(), FF::from(a_inv));
        assert_eq!(FF::from(a) * FF::from(a_inv), One::one());

        // associative
        let a = rng.sample(distr);
        let b = rng.sample(distr);
        let c = rng.sample(distr);
        assert_eq!(
            (FF::from(a) + FF::from(b)) + FF::from(c),
            FF::from(a) + (FF::from(b) + FF::from(c))
        );
        assert_eq!(
            (FF::from(a) * FF::from(b)) * FF::from(c),
            FF::from(a) * (FF::from(b) * FF::from(c))
        );

        // commutative
        let a = rng.sample(distr);
        let b = rng.sample(distr);
        assert_eq!(FF::from(a) + FF::from(b), FF::from(b) + FF::from(a));
        assert_eq!(FF::from(a) * FF::from(b), FF::from(b) * FF::from(a));

        // identity
        let a = rng.sample(distr);
        assert_eq!(FF::from(a) + FF::from(0), FF::from(a));
        assert_eq!(FF::from(a) * FF::from(1), FF::from(a));

        // distribute
        let a = rng.sample(distr);
        let b = rng.sample(distr);
        let c = rng.sample(distr);
        assert_eq!(
            (FF::from(a) + FF::from(b)) * FF::from(c),
            (FF::from(a) * FF::from(c)) + (FF::from(b) * FF::from(c))
        );
    }

    #[test]
    fn test_decompose() {
        const B: u32 = 1 << 2;
        let rng = &mut thread_rng();

        let a: Fp32 = rng.gen();
        let decompose = a.decompose(B);
        let compose = decompose
            .into_iter()
            .enumerate()
            .fold(Fp32(0), |acc, (i, d)| acc + d.mul_scalar(B.pow(i as u32)));

        assert_eq!(compose, a);
    }
}