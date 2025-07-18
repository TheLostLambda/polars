use std::ops::DerefMut;

use polars_compute::rolling::QuantileMethod;

use super::*;
use crate::chunked_array::comparison::*;
#[cfg(feature = "algorithm_group_by")]
use crate::frame::group_by::*;
use crate::prelude::*;

unsafe impl IntoSeries for DurationChunked {
    fn into_series(self) -> Series {
        Series(Arc::new(SeriesWrap(self)))
    }
}

impl private::PrivateSeriesNumeric for SeriesWrap<DurationChunked> {
    fn bit_repr(&self) -> Option<BitRepr> {
        Some(self.0.to_bit_repr())
    }
}

impl private::PrivateSeries for SeriesWrap<DurationChunked> {
    fn compute_len(&mut self) {
        self.0.compute_len()
    }
    fn _field(&self) -> Cow<'_, Field> {
        Cow::Owned(self.0.field())
    }
    fn _dtype(&self) -> &DataType {
        self.0.dtype()
    }

    fn _set_flags(&mut self, flags: StatisticsFlags) {
        self.0.deref_mut().set_flags(flags)
    }
    fn _get_flags(&self) -> StatisticsFlags {
        self.0.deref().get_flags()
    }

    unsafe fn equal_element(&self, idx_self: usize, idx_other: usize, other: &Series) -> bool {
        self.0.equal_element(idx_self, idx_other, other)
    }

    #[cfg(feature = "zip_with")]
    fn zip_with_same_type(&self, mask: &BooleanChunked, other: &Series) -> PolarsResult<Series> {
        let other = other.to_physical_repr().into_owned();
        self.0
            .zip_with(mask, other.as_ref().as_ref())
            .map(|ca| ca.into_duration(self.0.time_unit()).into_series())
    }

    fn into_total_eq_inner<'a>(&'a self) -> Box<dyn TotalEqInner + 'a> {
        self.0.physical().into_total_eq_inner()
    }
    fn into_total_ord_inner<'a>(&'a self) -> Box<dyn TotalOrdInner + 'a> {
        self.0.physical().into_total_ord_inner()
    }

    fn vec_hash(
        &self,
        random_state: PlSeedableRandomStateQuality,
        buf: &mut Vec<u64>,
    ) -> PolarsResult<()> {
        self.0.vec_hash(random_state, buf)?;
        Ok(())
    }

    fn vec_hash_combine(
        &self,
        build_hasher: PlSeedableRandomStateQuality,
        hashes: &mut [u64],
    ) -> PolarsResult<()> {
        self.0.vec_hash_combine(build_hasher, hashes)?;
        Ok(())
    }

    #[cfg(feature = "algorithm_group_by")]
    unsafe fn agg_min(&self, groups: &GroupsType) -> Series {
        self.0
            .agg_min(groups)
            .into_duration(self.0.time_unit())
            .into_series()
    }

    #[cfg(feature = "algorithm_group_by")]
    unsafe fn agg_max(&self, groups: &GroupsType) -> Series {
        self.0
            .agg_max(groups)
            .into_duration(self.0.time_unit())
            .into_series()
    }

    #[cfg(feature = "algorithm_group_by")]
    unsafe fn agg_sum(&self, groups: &GroupsType) -> Series {
        self.0
            .agg_sum(groups)
            .into_duration(self.0.time_unit())
            .into_series()
    }

    #[cfg(feature = "algorithm_group_by")]
    unsafe fn agg_std(&self, groups: &GroupsType, ddof: u8) -> Series {
        self.0
            .agg_std(groups, ddof)
            // cast f64 back to physical type
            .cast(&DataType::Int64)
            .unwrap()
            .into_duration(self.0.time_unit())
            .into_series()
    }

    #[cfg(feature = "algorithm_group_by")]
    unsafe fn agg_var(&self, groups: &GroupsType, ddof: u8) -> Series {
        self.0
            .agg_var(groups, ddof)
            // cast f64 back to physical type
            .cast(&DataType::Int64)
            .unwrap()
            .into_duration(self.0.time_unit())
            .into_series()
    }

    #[cfg(feature = "algorithm_group_by")]
    unsafe fn agg_list(&self, groups: &GroupsType) -> Series {
        // we cannot cast and dispatch as the inner type of the list would be incorrect
        self.0
            .agg_list(groups)
            .cast(&DataType::List(Box::new(self.dtype().clone())))
            .unwrap()
    }

    fn subtract(&self, rhs: &Series) -> PolarsResult<Series> {
        match (self.dtype(), rhs.dtype()) {
            (DataType::Duration(tu), DataType::Duration(tur)) => {
                polars_ensure!(tu == tur, InvalidOperation: "units are different");
                let lhs = self.cast(&DataType::Int64, CastOptions::NonStrict).unwrap();
                let rhs = rhs.cast(&DataType::Int64).unwrap();
                Ok(lhs.subtract(&rhs)?.into_duration(*tu).into_series())
            },
            (dtl, dtr) => polars_bail!(opq = sub, dtl, dtr),
        }
    }
    fn add_to(&self, rhs: &Series) -> PolarsResult<Series> {
        match (self.dtype(), rhs.dtype()) {
            (DataType::Duration(tu), DataType::Duration(tur)) => {
                polars_ensure!(tu == tur, InvalidOperation: "units are different");
                let lhs = self.cast(&DataType::Int64, CastOptions::NonStrict).unwrap();
                let rhs = rhs.cast(&DataType::Int64).unwrap();
                Ok(lhs.add_to(&rhs)?.into_duration(*tu).into_series())
            },
            (DataType::Duration(tu), DataType::Date) => {
                let one_day_in_tu: i64 = match tu {
                    TimeUnit::Milliseconds => 86_400_000,
                    TimeUnit::Microseconds => 86_400_000_000,
                    TimeUnit::Nanoseconds => 86_400_000_000_000,
                };
                let lhs =
                    self.cast(&DataType::Int64, CastOptions::NonStrict).unwrap() / one_day_in_tu;
                let rhs = rhs
                    .cast(&DataType::Int32)
                    .unwrap()
                    .cast(&DataType::Int64)
                    .unwrap();
                Ok(lhs
                    .add_to(&rhs)?
                    .cast(&DataType::Int32)?
                    .into_date()
                    .into_series())
            },
            (DataType::Duration(tu), DataType::Datetime(tur, tz)) => {
                polars_ensure!(tu == tur, InvalidOperation: "units are different");
                let lhs = self.cast(&DataType::Int64, CastOptions::NonStrict).unwrap();
                let rhs = rhs.cast(&DataType::Int64).unwrap();
                Ok(lhs
                    .add_to(&rhs)?
                    .into_datetime(*tu, tz.clone())
                    .into_series())
            },
            (dtl, dtr) => polars_bail!(opq = add, dtl, dtr),
        }
    }
    fn multiply(&self, rhs: &Series) -> PolarsResult<Series> {
        let tul = self.0.time_unit();
        match rhs.dtype() {
            DataType::Int64 => Ok((&self.0.phys * rhs.i64().unwrap())
                .into_duration(tul)
                .into_series()),
            dt if dt.is_integer() => {
                let rhs = rhs.cast(&DataType::Int64)?;
                self.multiply(&rhs)
            },
            dt if dt.is_float() => {
                let phys = &self.0.phys;
                let phys_float = phys.cast(dt).unwrap();
                let out = std::ops::Mul::mul(&phys_float, rhs)?
                    .cast(&DataType::Int64)
                    .unwrap();
                let phys = out.i64().unwrap().clone();
                Ok(phys.into_duration(tul).into_series())
            },
            _ => {
                polars_bail!(opq = mul, self.dtype(), rhs.dtype());
            },
        }
    }
    fn divide(&self, rhs: &Series) -> PolarsResult<Series> {
        let tul = self.0.time_unit();
        match rhs.dtype() {
            DataType::Duration(tur) => {
                if tul == *tur {
                    // Returns a constant as f64.
                    Ok(std::ops::Div::div(
                        &self.0.phys.cast(&DataType::Float64).unwrap(),
                        &rhs.duration()
                            .unwrap()
                            .phys
                            .cast(&DataType::Float64)
                            .unwrap(),
                    )?
                    .into_series())
                } else {
                    let rhs = rhs.cast(self.dtype())?;
                    self.divide(&rhs)
                }
            },
            DataType::Int64 => Ok((&self.0.phys / rhs.i64().unwrap())
                .into_duration(tul)
                .into_series()),
            dt if dt.is_integer() => {
                let rhs = rhs.cast(&DataType::Int64)?;
                self.divide(&rhs)
            },
            dt if dt.is_float() => {
                let phys = &self.0.phys;
                let phys_float = phys.cast(dt).unwrap();
                let out = std::ops::Div::div(&phys_float, rhs)?
                    .cast(&DataType::Int64)
                    .unwrap();
                let phys = out.i64().unwrap().clone();
                Ok(phys.into_duration(tul).into_series())
            },
            _ => {
                polars_bail!(opq = div, self.dtype(), rhs.dtype());
            },
        }
    }
    fn remainder(&self, rhs: &Series) -> PolarsResult<Series> {
        polars_ensure!(self.dtype() == rhs.dtype(), InvalidOperation: "dtypes and units must be equal in duration arithmetic");
        let lhs = self.cast(&DataType::Int64, CastOptions::NonStrict).unwrap();
        let rhs = rhs.cast(&DataType::Int64).unwrap();
        Ok(lhs
            .remainder(&rhs)?
            .into_duration(self.0.time_unit())
            .into_series())
    }
    #[cfg(feature = "algorithm_group_by")]
    fn group_tuples(&self, multithreaded: bool, sorted: bool) -> PolarsResult<GroupsType> {
        self.0.group_tuples(multithreaded, sorted)
    }

    fn arg_sort_multiple(
        &self,
        by: &[Column],
        options: &SortMultipleOptions,
    ) -> PolarsResult<IdxCa> {
        self.0.deref().arg_sort_multiple(by, options)
    }
}

impl SeriesTrait for SeriesWrap<DurationChunked> {
    fn rename(&mut self, name: PlSmallStr) {
        self.0.rename(name);
    }

    fn chunk_lengths(&self) -> ChunkLenIter<'_> {
        self.0.chunk_lengths()
    }
    fn name(&self) -> &PlSmallStr {
        self.0.name()
    }

    fn chunks(&self) -> &Vec<ArrayRef> {
        self.0.chunks()
    }
    unsafe fn chunks_mut(&mut self) -> &mut Vec<ArrayRef> {
        self.0.chunks_mut()
    }

    fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit()
    }

    fn slice(&self, offset: i64, length: usize) -> Series {
        self.0
            .slice(offset, length)
            .into_duration(self.0.time_unit())
            .into_series()
    }

    fn split_at(&self, offset: i64) -> (Series, Series) {
        let (a, b) = self.0.split_at(offset);
        let a = a.into_duration(self.0.time_unit()).into_series();
        let b = b.into_duration(self.0.time_unit()).into_series();
        (a, b)
    }

    fn _sum_as_f64(&self) -> f64 {
        self.0._sum_as_f64()
    }

    fn mean(&self) -> Option<f64> {
        self.0.mean()
    }

    fn median(&self) -> Option<f64> {
        self.0.median()
    }

    fn std(&self, ddof: u8) -> Option<f64> {
        self.0.std(ddof)
    }

    fn var(&self, ddof: u8) -> Option<f64> {
        self.0.var(ddof)
    }

    fn append(&mut self, other: &Series) -> PolarsResult<()> {
        polars_ensure!(self.0.dtype() == other.dtype(), append);
        let mut other = other.to_physical_repr().into_owned();
        self.0
            .append_owned(std::mem::take(other._get_inner_mut().as_mut()))
    }
    fn append_owned(&mut self, mut other: Series) -> PolarsResult<()> {
        polars_ensure!(self.0.dtype() == other.dtype(), append);
        self.0.append_owned(std::mem::take(
            &mut other
                ._get_inner_mut()
                .as_any_mut()
                .downcast_mut::<DurationChunked>()
                .unwrap()
                .phys,
        ))
    }

    fn extend(&mut self, other: &Series) -> PolarsResult<()> {
        polars_ensure!(self.0.dtype() == other.dtype(), extend);
        let other = other.to_physical_repr();
        self.0.extend(other.as_ref().as_ref().as_ref())?;
        Ok(())
    }

    fn filter(&self, filter: &BooleanChunked) -> PolarsResult<Series> {
        self.0
            .filter(filter)
            .map(|ca| ca.into_duration(self.0.time_unit()).into_series())
    }

    fn take(&self, indices: &IdxCa) -> PolarsResult<Series> {
        Ok(self
            .0
            .take(indices)?
            .into_duration(self.0.time_unit())
            .into_series())
    }

    unsafe fn take_unchecked(&self, indices: &IdxCa) -> Series {
        self.0
            .take_unchecked(indices)
            .into_duration(self.0.time_unit())
            .into_series()
    }

    fn take_slice(&self, indices: &[IdxSize]) -> PolarsResult<Series> {
        Ok(self
            .0
            .take(indices)?
            .into_duration(self.0.time_unit())
            .into_series())
    }

    unsafe fn take_slice_unchecked(&self, indices: &[IdxSize]) -> Series {
        self.0
            .take_unchecked(indices)
            .into_duration(self.0.time_unit())
            .into_series()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn rechunk(&self) -> Series {
        self.0
            .rechunk()
            .into_owned()
            .into_duration(self.0.time_unit())
            .into_series()
    }

    fn new_from_index(&self, index: usize, length: usize) -> Series {
        self.0
            .new_from_index(index, length)
            .into_duration(self.0.time_unit())
            .into_series()
    }

    fn cast(&self, dtype: &DataType, cast_options: CastOptions) -> PolarsResult<Series> {
        self.0.cast_with_options(dtype, cast_options)
    }

    #[inline]
    unsafe fn get_unchecked(&self, index: usize) -> AnyValue<'_> {
        self.0.get_any_value_unchecked(index)
    }

    fn sort_with(&self, options: SortOptions) -> PolarsResult<Series> {
        Ok(self
            .0
            .sort_with(options)
            .into_duration(self.0.time_unit())
            .into_series())
    }

    fn arg_sort(&self, options: SortOptions) -> IdxCa {
        self.0.arg_sort(options)
    }

    fn null_count(&self) -> usize {
        self.0.null_count()
    }

    fn has_nulls(&self) -> bool {
        self.0.has_nulls()
    }

    #[cfg(feature = "algorithm_group_by")]
    fn unique(&self) -> PolarsResult<Series> {
        self.0
            .unique()
            .map(|ca| ca.into_duration(self.0.time_unit()).into_series())
    }

    #[cfg(feature = "algorithm_group_by")]
    fn n_unique(&self) -> PolarsResult<usize> {
        self.0.n_unique()
    }

    #[cfg(feature = "algorithm_group_by")]
    fn arg_unique(&self) -> PolarsResult<IdxCa> {
        self.0.arg_unique()
    }

    fn is_null(&self) -> BooleanChunked {
        self.0.is_null()
    }

    fn is_not_null(&self) -> BooleanChunked {
        self.0.is_not_null()
    }

    fn reverse(&self) -> Series {
        self.0
            .reverse()
            .into_duration(self.0.time_unit())
            .into_series()
    }

    fn as_single_ptr(&mut self) -> PolarsResult<usize> {
        self.0.as_single_ptr()
    }

    fn shift(&self, periods: i64) -> Series {
        self.0
            .shift(periods)
            .into_duration(self.0.time_unit())
            .into_series()
    }

    fn sum_reduce(&self) -> PolarsResult<Scalar> {
        let sc = self.0.sum_reduce();
        let v = sc.value().as_duration(self.0.time_unit());
        Ok(Scalar::new(self.dtype().clone(), v))
    }

    fn max_reduce(&self) -> PolarsResult<Scalar> {
        let sc = self.0.max_reduce();
        let v = sc.value().as_duration(self.0.time_unit());
        Ok(Scalar::new(self.dtype().clone(), v))
    }
    fn min_reduce(&self) -> PolarsResult<Scalar> {
        let sc = self.0.min_reduce();
        let v = sc.value().as_duration(self.0.time_unit());
        Ok(Scalar::new(self.dtype().clone(), v))
    }
    fn std_reduce(&self, ddof: u8) -> PolarsResult<Scalar> {
        let sc = self.0.std_reduce(ddof);
        let to = self.dtype().to_physical();
        let v = sc.value().cast(&to);
        Ok(Scalar::new(
            self.dtype().clone(),
            v.as_duration(self.0.time_unit()),
        ))
    }

    fn var_reduce(&self, ddof: u8) -> PolarsResult<Scalar> {
        // Why do we go via MilliSeconds here? Seems wrong to me.
        // I think we should fix/inspect the tests that fail if we remain on the time-unit here.
        let sc = self
            .0
            .cast_time_unit(TimeUnit::Milliseconds)
            .var_reduce(ddof);
        let to = self.dtype().to_physical();
        let v = sc.value().cast(&to);
        Ok(Scalar::new(
            DataType::Duration(TimeUnit::Milliseconds),
            v.as_duration(TimeUnit::Milliseconds),
        ))
    }
    fn median_reduce(&self) -> PolarsResult<Scalar> {
        let v: AnyValue = self.median().map(|v| v as i64).into();
        let to = self.dtype().to_physical();
        let v = v.cast(&to);
        Ok(Scalar::new(
            self.dtype().clone(),
            v.as_duration(self.0.time_unit()),
        ))
    }
    fn quantile_reduce(&self, quantile: f64, method: QuantileMethod) -> PolarsResult<Scalar> {
        let v = self.0.quantile_reduce(quantile, method)?;
        let to = self.dtype().to_physical();
        let v = v.value().cast(&to);
        Ok(Scalar::new(
            self.dtype().clone(),
            v.as_duration(self.0.time_unit()),
        ))
    }

    fn clone_inner(&self) -> Arc<dyn SeriesTrait> {
        Arc::new(SeriesWrap(Clone::clone(&self.0)))
    }

    fn find_validity_mismatch(&self, other: &Series, idxs: &mut Vec<IdxSize>) {
        self.0.find_validity_mismatch(other, idxs)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut self.0
    }

    fn as_phys_any(&self) -> &dyn Any {
        self.0.physical()
    }

    fn as_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self as _
    }
}
