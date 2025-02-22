use bitvec::prelude::BitSlice;

use crate::spaces::tools::peek_top_largest_iterable;
use crate::types::{PointOffsetType, ScoreType};
use crate::vector_storage::{RawScorer, ScoredPointOffset};

pub struct QuantizedRawScorer<'a, TEncodedQuery, TEncodedVectors>
where
    TEncodedVectors: quantization::EncodedVectors<TEncodedQuery>,
{
    pub(super) query: TEncodedQuery,
    /// [`BitSlice`] defining flags for deleted points (and thus these vectors).
    pub(super) point_deleted: &'a BitSlice,
    /// [`BitSlice`] defining flags for deleted vectors in this segment.
    pub(super) vec_deleted: &'a BitSlice,
    pub quantized_data: &'a TEncodedVectors,
}

impl<TEncodedQuery, TEncodedVectors> RawScorer
    for QuantizedRawScorer<'_, TEncodedQuery, TEncodedVectors>
where
    TEncodedVectors: quantization::EncodedVectors<TEncodedQuery>,
{
    fn score_points(&self, points: &[PointOffsetType], scores: &mut [ScoredPointOffset]) -> usize {
        let mut size: usize = 0;
        for point_id in points.iter().copied() {
            if !self.check_vector(point_id) {
                continue;
            }
            scores[size] = ScoredPointOffset {
                idx: point_id,
                score: self.quantized_data.score_point(&self.query, point_id),
            };
            size += 1;
            if size == scores.len() {
                return size;
            }
        }
        size
    }

    fn check_vector(&self, point: PointOffsetType) -> bool {
        // Deleted points propagate to vectors; check vector deletion for possible early return
        !self
            .vec_deleted
            .get(point as usize)
            .as_deref()
            .copied()
            .unwrap_or(false)
        // Additionally check point deletion for integrity if delete propagation to vector failed
        && !self
            .point_deleted
            .get(point as usize)
            .as_deref()
            .copied()
            .unwrap_or(false)
    }

    fn score_point(&self, point: PointOffsetType) -> ScoreType {
        self.quantized_data.score_point(&self.query, point)
    }

    fn score_internal(&self, point_a: PointOffsetType, point_b: PointOffsetType) -> ScoreType {
        self.quantized_data.score_internal(point_a, point_b)
    }

    fn peek_top_iter(
        &self,
        points: &mut dyn Iterator<Item = PointOffsetType>,
        top: usize,
    ) -> Vec<ScoredPointOffset> {
        let scores = points.filter(|idx| self.check_vector(*idx)).map(|idx| {
            let score = self.score_point(idx);
            ScoredPointOffset { idx, score }
        });
        peek_top_largest_iterable(scores, top)
    }

    fn peek_top_all(&self, top: usize) -> Vec<ScoredPointOffset> {
        let scores = (0..self.point_deleted.len() as PointOffsetType)
            .filter(|idx| self.check_vector(*idx))
            .map(|idx| {
                let score = self.score_point(idx);
                ScoredPointOffset { idx, score }
            });
        peek_top_largest_iterable(scores, top)
    }
}
