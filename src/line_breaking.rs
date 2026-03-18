use crate::boxes::{
    set_glue_for_dimen, BoxLayout, GlueSetResult, HorizontalBox, TeXBox,
};
use crate::dimension::Dimen;
use crate::glue::Glue;
use crate::list::HorizontalListElem;
use crate::state::TeXState;

use std::collections::HashMap;

pub struct LineBreakingParams {
    pub hsize: Dimen,
    pub tolerance: i32,
    pub visual_incompatibility_demerits: i32,

    // Whether we should log information about the line breaking procedure. Set
    // by \tracingparagraphs
    pub should_log: bool,
}

#[derive(Debug, PartialEq)]
struct LineBreakingResult {
    total_demerits: i64,
    all_breaks: Vec<LineBreakPoint>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
enum LineBreakPoint {
    Start,
    BreakAtIndex(usize),
    End,
}

#[cfg(test)]
mod line_break_point_tests {
    use super::LineBreakPoint;

    #[test]
    fn it_orders_line_break_points_correctly() {
        assert!(LineBreakPoint::Start < LineBreakPoint::BreakAtIndex(0));
        assert!(LineBreakPoint::Start < LineBreakPoint::BreakAtIndex(100));
        assert!(
            LineBreakPoint::BreakAtIndex(0) < LineBreakPoint::BreakAtIndex(1)
        );
        assert!(
            LineBreakPoint::BreakAtIndex(100)
                < LineBreakPoint::BreakAtIndex(101)
        );
        assert!(LineBreakPoint::Start < LineBreakPoint::End);
        assert!(LineBreakPoint::BreakAtIndex(0) < LineBreakPoint::End);
        assert!(LineBreakPoint::BreakAtIndex(101) < LineBreakPoint::End);
    }
}

fn get_list_indices_for_breaks(
    list: &[HorizontalListElem],
    start: &LineBreakPoint,
    end: &LineBreakPoint,
) -> Option<(usize, usize)> {
    let start_index = match start {
        LineBreakPoint::Start => Some(0),
        LineBreakPoint::BreakAtIndex(index) => Some(
            list.iter()
                .skip(*index)
                .position(|elem| !elem.is_discardable())?
                + index,
        ),
        _ => None,
    }?;

    let end_index = match end {
        LineBreakPoint::End => Some(list.len()),
        LineBreakPoint::BreakAtIndex(index) => Some(*index),
        _ => None,
    }?;

    Some((start_index, end_index))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum VisualClassification {
    VeryLoose = 0,
    Loose = 1,
    Decent = 2,
    Tight = 3,
}

impl VisualClassification {
    fn all_ordered_classifications() -> [VisualClassification; 4] {
        [
            VisualClassification::VeryLoose,
            VisualClassification::Loose,
            VisualClassification::Decent,
            VisualClassification::Tight,
        ]
    }

    fn is_adjacent(&self, other: &VisualClassification) -> bool {
        match (self, other) {
            (VisualClassification::VeryLoose, VisualClassification::Decent) => {
                false
            }
            (VisualClassification::VeryLoose, VisualClassification::Tight) => {
                false
            }
            (VisualClassification::Loose, VisualClassification::Tight) => false,
            (VisualClassification::Decent, VisualClassification::VeryLoose) => {
                false
            }
            (VisualClassification::Tight, VisualClassification::Loose) => false,
            (VisualClassification::Tight, VisualClassification::VeryLoose) => {
                false
            }
            (_, _) => true,
        }
    }

    fn from_badness(badness: u64, shrink: bool) -> VisualClassification {
        if badness >= 100 {
            VisualClassification::VeryLoose
        } else if badness >= 13 {
            if shrink {
                VisualClassification::Tight
            } else {
                VisualClassification::Loose
            }
        } else {
            VisualClassification::Decent
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LineBreak {
    line_break_point: LineBreakPoint,
    classification_before_break: VisualClassification,
}

#[derive(Debug, Clone)]
struct LineBreakBacktrace {
    prev_break: Option<LineBreak>,
    total_demerits: i64,
}

#[derive(Debug)]
struct LineBreakGraph {
    // A map to keep track of the potential best ways to get to a certain break
    // point. It maps from a given point in the list (and the visual
    // classification that it arrived at) to the best previous line break (and
    // also the demerits that would be accrued if that line was set).
    best_path_to: HashMap<LineBreak, LineBreakBacktrace>,
}

impl LineBreakGraph {
    // Set up an empty line breaking graph given a list of indices.
    fn new() -> Self {
        let mut graph = LineBreakGraph {
            best_path_to: HashMap::new(),
        };

        graph.best_path_to.insert(
            LineBreak {
                line_break_point: LineBreakPoint::Start,
                classification_before_break: VisualClassification::Decent,
            },
            LineBreakBacktrace {
                prev_break: None,
                total_demerits: 0,
            },
        );

        graph
    }

    // Find the best demerits from the start to a given node, if one exists
    fn get_best_demerits_to_node(&self, to: &LineBreak) -> Option<i64> {
        self.best_path_to
            .get(to)
            .map(|backtrace| backtrace.total_demerits)
    }

    fn update_best_path_to_node(
        &mut self,
        to: LineBreak,
        from: LineBreak,
        demerits: i64,
    ) {
        self.best_path_to.insert(
            to,
            LineBreakBacktrace {
                prev_break: Some(from),
                total_demerits: demerits,
            },
        );
    }

    fn get_best_backtrace_to_point(
        &self,
        to: &LineBreakPoint,
    ) -> Option<&LineBreakBacktrace> {
        // There might be multiple breaking options which all arrive at the same
        // point, but have different visual classifications. Search for all of
        // those options and choose the best.
        let mut backtraces =
            VisualClassification::all_ordered_classifications()
                .iter()
                .filter_map(|visual_classification| {
                    self.best_path_to.get(&LineBreak {
                        line_break_point: *to,
                        classification_before_break: *visual_classification,
                    })
                })
                .collect::<Vec<_>>();

        backtraces.sort_by(|a, b| a.total_demerits.cmp(&b.total_demerits));
        backtraces.first().copied()
    }

    fn get_best_breaks_to_end(&self) -> Option<LineBreakingResult> {
        let mut all_breaks = vec![LineBreakPoint::End];
        let mut curr_break_backtrace =
            self.get_best_backtrace_to_point(&LineBreakPoint::End)?;

        let end_demerits = curr_break_backtrace.total_demerits;

        while let Some(prev_break) = &curr_break_backtrace.prev_break {
            all_breaks.insert(0, prev_break.line_break_point);
            curr_break_backtrace = self.best_path_to.get(prev_break)?;
        }

        Some(LineBreakingResult {
            total_demerits: end_demerits,
            all_breaks,
        })
    }
}

fn can_break_at_index(list: &[HorizontalListElem], index: usize) -> bool {
    if index == list.len() - 1 {
        // We treat LineBreakPoint::End as the final element, so don't allow a
        // separate line break at the final element itself.
        return false;
    }

    match list[index] {
        HorizontalListElem::HSkip(_) => {
            index == 0 || !list[index - 1].is_discardable()
        }
        HorizontalListElem::Penalty(_) => true,
        _ => false,
    }
}

fn get_available_break_indices(
    list: &[HorizontalListElem],
) -> Vec<LineBreakPoint> {
    let mut available_break_indices = Vec::new();

    available_break_indices.push(LineBreakPoint::Start);
    available_break_indices.extend(
        (0..list.len())
            .filter(|i| can_break_at_index(list, *i))
            .map(LineBreakPoint::BreakAtIndex),
    );
    available_break_indices.push(LineBreakPoint::End);

    available_break_indices
}

fn get_penalty_at_point(
    list: &[HorizontalListElem],
    point: &LineBreakPoint,
) -> i64 {
    match point {
        LineBreakPoint::Start => 0,
        // We treat the final element in the list as the end
        LineBreakPoint::End => list
            .last()
            .map(|elem| elem.get_penalty() as i64)
            .unwrap_or(0),
        LineBreakPoint::BreakAtIndex(index) => {
            list[*index].get_penalty() as i64
        }
    }
}

#[derive(Debug)]
enum DemeritResult {
    Overfull,
    CannotBreak,
    Demerits {
        demerits: i64,
        penalty: i64,
        badness: u64,
        classification: VisualClassification,
    },
}

fn get_demerits_for_line_between(
    list: &[HorizontalListElem],
    params: &LineBreakingParams,
    state: &TeXState,
    start: &LineBreakPoint,
    end: &LineBreakPoint,
    previous_classification: Option<VisualClassification>,
) -> Option<DemeritResult> {
    let (start_index, end_index) =
        get_list_indices_for_breaks(list, start, end)?;

    if start_index > end_index {
        return None;
    }

    let penalty: i64 = get_penalty_at_point(list, end);

    if penalty >= 10000 {
        return Some(DemeritResult::CannotBreak);
    }

    let line_width = list
        .get(start_index..end_index)?
        .iter()
        .fold(Glue::zero(), |width, elem| width + elem.get_size(state).2);

    let glue_set = set_glue_for_dimen(&params.hsize, &line_width);
    let badness = match glue_set {
        GlueSetResult::GlueSetRatio(glue_set_ratio) => {
            glue_set_ratio.get_badness()
        }
        GlueSetResult::InsufficientShrink => {
            return Some(DemeritResult::Overfull);
        }
        // We treat underfull boxes as 10000 badness. This lets us still set
        // underfull boxes if \tolerance=10000
        GlueSetResult::ZeroStretch => 10000,
        GlueSetResult::ZeroShrink => {
            return Some(DemeritResult::Overfull);
        }
    };

    if badness > params.tolerance as u64 && penalty > -10000 {
        return Some(DemeritResult::CannotBreak);
    }

    let visual_classification = VisualClassification::from_badness(
        badness,
        params.hsize < line_width.space,
    );
    let adjacent_classification_demerits =
        if let Some(previous_classification) = previous_classification {
            if visual_classification.is_adjacent(&previous_classification) {
                0
            } else {
                params.visual_incompatibility_demerits as i64
            }
        } else {
            0
        };

    let additional_demerits: i64 = adjacent_classification_demerits;

    let line_penalty: i64 = 10;

    let base_demerits = (line_penalty + badness as i64).min(10_000).pow(2);

    let penalty_adjustment_demerits = if (0..10_000).contains(&penalty) {
        penalty.pow(2)
    } else if (-9_999..0).contains(&penalty) {
        -penalty.pow(2)
    } else if penalty >= 10_000 {
        return None;
    } else {
        // penalty <= -10000
        0
    };

    Some(DemeritResult::Demerits {
        demerits: base_demerits
            + penalty_adjustment_demerits
            + additional_demerits,
        badness,
        penalty,
        classification: visual_classification,
    })
}

#[derive(Clone)]
struct BackwardsPath {
    line_break: LineBreak,
    total_demerits: i64,
}

// Given a horizontal list, try to generate the best line breaks which match the
// line breaking params.
fn generate_best_list_break_option_with_params(
    list: &[HorizontalListElem],
    params: &LineBreakingParams,
    state: &TeXState,
) -> Option<LineBreakingResult> {
    // This function implements the Knuth-Plass line breaking algorithm. This is
    // an optimized version of a shortest path graph search, where each
    // available break point is a node and the weight of the edges between them
    // is the badness of setting the line between those break points.

    let line_breaks = get_available_break_indices(list);
    let mut graph = LineBreakGraph::new();

    // Keep track of previous breakpoints that we've looked at already, that are
    // still reachable from the current break without being overfull.
    let mut reachable_previous_breaks: Vec<LineBreak> =
        Vec::from([LineBreak {
            line_break_point: LineBreakPoint::Start,
            classification_before_break: VisualClassification::Decent,
        }]);

    // For logging, we don't want to refer to our `LineBreak`s using our
    // internal representation, so we sequentially number the feasible
    // breakpoints we find, with the start referring to 0.
    let mut next_feasible_line_break_number = 1;
    let mut feasible_line_break_numbers: HashMap<LineBreak, usize> =
        HashMap::new();
    feasible_line_break_numbers.insert(
        LineBreak {
            line_break_point: LineBreakPoint::Start,
            classification_before_break: VisualClassification::Decent,
        },
        0,
    );

    for line_break_point in line_breaks.iter().skip(1) {
        let mut best_backwards_paths_by_classification: HashMap<
            VisualClassification,
            BackwardsPath,
        > = HashMap::new();

        for previous_break in reachable_previous_breaks.clone().iter() {
            let previous_demerits =
                graph.get_best_demerits_to_node(previous_break).unwrap();
            if let Some(demerits) = get_demerits_for_line_between(
                list,
                params,
                state,
                &previous_break.line_break_point,
                line_break_point,
                Some(previous_break.classification_before_break),
            ) {
                match demerits {
                    DemeritResult::Overfull => {
                        // If we reach a previously visited breakpoint where
                        // setting the line between it and the current break
                        // would overfill the line, we no longer want to look at
                        // that previous break. Remove it from the list of
                        // previous breaks.

                        // We have to look up the new position of the current
                        // break because other breaks might have been removed so
                        // the original index of that node might have changed.
                        let previous_break_index = reachable_previous_breaks
                            .iter()
                            .position(|bp| bp == previous_break)
                            .unwrap();
                        reachable_previous_breaks.remove(previous_break_index);

                        if reachable_previous_breaks.is_empty() {
                            // In a very special case where removing the
                            // previous break would remove all of the previous
                            // viable breakpoints, this means that there are no
                            // possible ways to break the line while staying
                            // within our constraints.
                            //
                            // Instead, we add an overfull line between the
                            // current node and the previous break we are
                            // currently looking at. Because
                            // `reachable_previous_breaks` is sorted and all of
                            // the other elements were removed, we know that the
                            // previous break we are looking at will be the
                            // furthest along break, which will produce the
                            // smallest overfull line.
                            if params.should_log {
                                println!(
                                    "@ via @@{:?} b=* p=0 d=*",
                                    feasible_line_break_numbers[previous_break]
                                );
                            }
                            best_backwards_paths_by_classification.insert(
                                VisualClassification::Tight,
                                BackwardsPath {
                                    line_break: previous_break.clone(),
                                    // When this happens, even though this is a very bad
                                    // situation, we add no demerits.
                                    total_demerits: previous_demerits,
                                },
                            );
                        }
                    }
                    DemeritResult::CannotBreak => {} // ignore
                    DemeritResult::Demerits {
                        mut demerits,
                        badness,
                        penalty,
                        classification,
                    } => {
                        let mut demerits_display = format!("{}", demerits);

                        if penalty <= -10000 {
                            if reachable_previous_breaks.len() == 1 {
                                // If a \penalty-10000 is the only viable
                                // breakpoint for a given line, then we don't
                                // add demerits for that line.
                                demerits = 0;

                                // When that happens, the log shows d=* instead of d=0.
                                demerits_display = "*".to_string();
                            }

                            // A \penalty-10000 forces us to break at that
                            // point. We accomplish that by removing all of the
                            // previous reachable points, so that any future
                            // possible breakpoints will only be able to look
                            // back this current breakpoint.
                            reachable_previous_breaks.retain(
                                |previous_break: &LineBreak| {
                                    previous_break.line_break_point
                                        >= *line_break_point
                                },
                            );
                        }

                        if params.should_log {
                            println!(
                                "@ via @@{:?} b={} p={} d={}",
                                feasible_line_break_numbers[previous_break],
                                badness,
                                penalty,
                                demerits_display
                            );
                        }

                        let current_backwards_path = BackwardsPath {
                            line_break: previous_break.clone(),
                            total_demerits: demerits + previous_demerits,
                        };

                        best_backwards_paths_by_classification
                            .entry(classification)
                            .and_modify(|previous_best_path| {
                                if current_backwards_path.total_demerits
                                    <= previous_best_path.total_demerits
                                {
                                    *previous_best_path =
                                        current_backwards_path.clone();
                                }
                            })
                            .or_insert(current_backwards_path);
                    }
                }
            }
        }

        let best_demerits = best_backwards_paths_by_classification
            .values()
            .map(|path| path.total_demerits)
            .min();

        // Only accept backwards paths that are as good as the best demerits we
        // found for a given starting point.
        let allowed_backwards_paths_by_classification: HashMap<
            VisualClassification,
            BackwardsPath,
        > = best_backwards_paths_by_classification
            .into_iter()
            .filter(|(_, v)| Some(v.total_demerits) == best_demerits)
            .collect();

        // We don't want to depend on the nondeterministic order of iterating
        // through the map of backwards paths. Instead, always iterate through
        // them in a specific order of classifications.
        for classification in
            VisualClassification::all_ordered_classifications()
        {
            if let Some(best_backwards_path) =
                allowed_backwards_paths_by_classification.get(&classification)
            {
                let line_break = LineBreak {
                    line_break_point: *line_break_point,
                    classification_before_break: classification,
                };

                feasible_line_break_numbers.insert(
                    line_break.clone(),
                    next_feasible_line_break_number,
                );
                next_feasible_line_break_number += 1;

                if params.should_log {
                    // TODO(xymostech): Keep track of the line number of a given active
                    // node to print here.
                    println!(
                        "@@{:?}: line x.{} t={} -> @@{:?}",
                        feasible_line_break_numbers[&line_break],
                        classification as u8,
                        best_backwards_path.total_demerits,
                        feasible_line_break_numbers
                            [&best_backwards_path.line_break]
                    );
                }

                graph.update_best_path_to_node(
                    line_break.clone(),
                    best_backwards_path.line_break.clone(),
                    best_backwards_path.total_demerits,
                );
                reachable_previous_breaks.push(line_break);
            }
        }
    }

    graph.get_best_breaks_to_end()
}

pub fn break_horizontal_list_to_lines_with_params(
    list: &[HorizontalListElem],
    params: LineBreakingParams,
    state: &TeXState,
) -> Option<Vec<TeXBox>> {
    let best_option =
        generate_best_list_break_option_with_params(list, &params, state)?;

    let break_pairs = best_option
        .all_breaks
        .iter()
        .zip(best_option.all_breaks.iter().skip(1));
    let line_boxes = break_pairs
        .map(|(start, end)| {
            let (start_index, end_index) =
                get_list_indices_for_breaks(list, start, end).unwrap();
            let line_list = &list.get(start_index..end_index).unwrap();
            let line_box =
                HorizontalBox::create_from_horizontal_list_with_layout(
                    Vec::from(*line_list),
                    &BoxLayout::Fixed(params.hsize),
                    state,
                );
            TeXBox::HorizontalBox(line_box)
        })
        .collect::<Vec<_>>();

    Some(line_boxes)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dimension::{FilDimen, FilKind, SpringDimen, Unit};
    use crate::testing::with_parser;

    fn expect_paragraph_to_parse_to_lines(
        paragraph: &[&str],
        lines: &[&str],
        params: LineBreakingParams,
        expected_demerits: i64,
    ) {
        with_parser(lines, |parser| {
            while parser.is_assignment_head() {
                parser.parse_assignment(None);
            }
            let mut expected_lines = Vec::new();
            while parser.is_box_head() {
                expected_lines.push(parser.parse_box().unwrap());
            }

            with_parser(paragraph, |parser| {
                let mut hlist = parser.parse_horizontal_list(false, false);

                if matches!(hlist.last(), Some(HorizontalListElem::HSkip(_))) {
                    hlist.pop();
                }

                hlist.extend_from_slice(&[
                    HorizontalListElem::Penalty(10000),
                    HorizontalListElem::HSkip(Glue {
                        space: Dimen::zero(),
                        stretch: SpringDimen::FilDimen(FilDimen::new(
                            FilKind::Fil,
                            1.0,
                        )),
                        shrink: SpringDimen::Dimen(Dimen::zero()),
                    }),
                    HorizontalListElem::Penalty(-10000),
                ]);

                let best_break = generate_best_list_break_option_with_params(
                    &hlist,
                    &params,
                    parser.state,
                )
                .unwrap();

                assert_eq!(best_break.total_demerits, expected_demerits);

                let actual_boxes = break_horizontal_list_to_lines_with_params(
                    &hlist,
                    LineBreakingParams {
                        // Since we run the algorithm twice, only log during one of the runs
                        should_log: false,
                        ..params
                    },
                    parser.state,
                )
                .unwrap();

                // If the assert below fails, the log isn't going to be super
                // helpful. Add in a slightly nicer check beforehand to give a
                // hint what went wrong.
                for (index, (actual_box, expected_line)) in
                    actual_boxes.iter().zip(expected_lines.iter()).enumerate()
                {
                    if actual_box != expected_line {
                        println!("First different line: {}", index);
                        println!(
                            "actual:   {:?} elems\nvs\nexpected: {:?} elems",
                            actual_box.to_chars(),
                            expected_line.to_chars()
                        );

                        if let (
                            TeXBox::HorizontalBox(actual_hbox),
                            TeXBox::HorizontalBox(expected_hbox),
                        ) = (actual_box, expected_line)
                        {
                            if actual_hbox.list.len()
                                != expected_hbox.list.len()
                            {
                                println!(
                                    "actual len: {} vs expected len: {}",
                                    actual_hbox.list.len(),
                                    expected_hbox.list.len()
                                );
                            } else {
                                for (index, (actual_elem, expected_elem)) in
                                    actual_hbox
                                        .list
                                        .iter()
                                        .zip(expected_hbox.list.iter())
                                        .enumerate()
                                {
                                    if actual_elem != expected_elem {
                                        println!(
                                            "First different elem: {}",
                                            index
                                        );
                                        println!("actual:   {:?}\nvs\nexpected: {:?}", actual_elem, expected_elem);

                                        break;
                                    }
                                }
                            }
                        }

                        break;
                    }
                }

                assert!(
                    actual_boxes == expected_lines,
                    "assertion failed: Lines didn't match up!"
                );
            });
        });
    }

    #[test]
    fn test_single_line_splitting() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"{\a} {\a\a\a\a} {\a\a}%",
            ],
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"\hbox to150pt{{\a} {\a\a\a\a} {\a\a}\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(150.0, Unit::Point),
                tolerance: 10000,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            100,
        );

        expect_paragraph_to_parse_to_lines(
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"{\a} {\a\a\a\a} {\a\a}%",
            ],
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"\hbox to105pt{{\a} {\a\a\a\a}}%",
                r"\hbox to105pt{{\a\a}\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(105.0, Unit::Point),
                tolerance: 10000,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            12100,
        );
    }

    #[test]
    fn test_whole_paragraph_splitting() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}%",
            ],
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"\def\line#1{\hbox to105pt{#1}}%",
                r"\line{{\a} \a\a\a\a}%",
                r"\line{{\a\a} \a\a\a}%",
                r"\line{{\a\a\a} \a\a}%",
                r"\line{{\a\a\a\a} \a}%",
                r"\line{{\a\a\a}\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(105.0, Unit::Point),
                tolerance: 10000,
                visual_incompatibility_demerits: 10000,
                should_log: true,
            },
            22100 + 12100 + 12100 + 12100 + 10100,
        );
    }

    #[test]
    fn test_long_paragraph_splitting() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}%",
            ],
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"\def\line#1{\hbox to400pt{#1}}%",
                r"\line{{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a}}%",
                r"\line{{\a} {\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a}}%",
                r"\line{{\a\a\a\a} {\a} {\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a}}%",
                r"\line{{\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a} {\a} {\a\a\a\a}}%",
                r"\line{{\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}}%",
                r"\line{{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a}}%",
                r"\line{{\a} {\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a}}%",
                r"\line{{\a\a\a\a} {\a} {\a\a\a}\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(400.0, Unit::Point),
                tolerance: 10000,
                visual_incompatibility_demerits: 10000,
                should_log: true,
            },
            100 + 324 + 666100 + 656100 + 656100 + 10100 + 324 + 100,
        );
    }

    #[test]
    fn it_splits_paragraphs_with_boxes_wider_than_hsize() {
        expect_paragraph_to_parse_to_lines(
            &[r"\hbox to90pt{ab\hskip0pt plus1fil cd} %", r"efg hij%"],
            &[
                r"\def\line#1{\hbox to80pt{#1}}%",
                r"\line{\hbox to90pt{ab\hskip0pt plus1fil cd}}%",
                r"\line{efg hij\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(80.0, Unit::Point),
                tolerance: 10000,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            100,
        );
    }

    #[test]
    fn it_splits_paragraphs_into_overfull_boxes_if_badness_is_low_enough() {
        let paragraph = [
            r"\def\sp{\hskip 1pt plus3pt{}}%",
            r"\def\box{\hbox to50pt{a}}%",
            r"\box\sp\box\sp\box\sp\box\sp\box%",
        ];

        expect_paragraph_to_parse_to_lines(
            &paragraph,
            &[
                r"\def\line#1{\hbox to110pt{#1}}%",
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\line{\box\sp\box}%",
                r"\line{\box\sp\box}%",
                r"\line{\box\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(110.0, Unit::Point),
                tolerance: 2700,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            7333264 + 7333264,
        );

        expect_paragraph_to_parse_to_lines(
            &paragraph,
            &[
                r"\def\line#1{\hbox to110pt{#1}}%",
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\line{\box\sp\box\sp\box}%",
                r"\line{\box\sp\box\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(110.0, Unit::Point),
                tolerance: 2600,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            0,
        );
    }

    #[test]
    fn it_treats_10000_tolerance_as_infinite() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                // Setting this to 120pt has a very large badness because the
                // spaces would need to stretch significantly. Instead of
                // allowing that badness, we get an overfull box.
                r"\box\sp\box\sp\box\sp\box\sp\box%",
            ],
            &[
                r"\def\line#1{\hbox to120pt{#1}}%",
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\line{\box\sp\box\sp\box}%",
                r"\line{\box\sp\box\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(120.0, Unit::Point),
                tolerance: 9999,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            0,
        );

        expect_paragraph_to_parse_to_lines(
            &[
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                // Even though this needs to stretch a huge amount, the
                // tolerance is infinite so this is allowed
                r"\box\sp\box\sp\box\sp\box\sp\box%",
            ],
            &[
                r"\def\line#1{\hbox to120pt{#1}}%",
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\line{\box\sp\box}%",
                r"\line{\box\sp\box}%",
                r"\line{\box\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(120.0, Unit::Point),
                tolerance: 10000,
                visual_incompatibility_demerits: 10000,
                should_log: true,
            },
            100010000 + 100000000 + 10100,
        );
    }

    #[test]
    fn it_considers_visual_incompatibility_when_making_linebreaks() {
        let paragraph = [
            r"\def\x{\hbox to20pt{x}}%",
            r"\def\spa{\hskip6pt plus2pt minus3.5pt}%",
            r"\def\spb{\hskip5pt plus15pt minus2.5pt}%",
            r"\x\spb\x\spb\x\spa%",
            r"\x\spa\x\spa\x\spa\x\spb%",
            r"\x\spb\x\spb\x\spa%",
            r"\x\spa\x\spa\x\spa\x\spb%",
            r"\x\spb\x\spb\x%",
        ];

        expect_paragraph_to_parse_to_lines(
            &paragraph,
            &[
                r"\def\x{\hbox to20pt{x}}%",
                r"\def\spa{\hskip6pt plus2pt minus3.5pt}%",
                r"\def\spb{\hskip5pt plus15pt minus2.5pt}%",
                r"\def\line#1{\hbox to90pt{#1}}%",
                r"\line{\x\spb\x\spb\x}%",
                r"\line{\x\spa\x\spa\x\spa\x}%",
                r"\line{\x\spb\x\spb\x}%",
                r"\line{\x\spa\x\spa\x\spa\x}%",
                r"\line{\x\spb\x\spb\x\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(90.0, Unit::Point),
                tolerance: 100,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            9132,
        );

        expect_paragraph_to_parse_to_lines(
            &paragraph,
            &[
                r"\def\x{\hbox to20pt{x}}%",
                r"\def\spa{\hskip6pt plus2pt minus3.5pt}%",
                r"\def\spb{\hskip5pt plus15pt minus2.5pt}%",
                r"\def\line#1{\hbox to90pt{#1}}%",
                r"\line{\x\spb\x\spb\x\spa\x}%",
                r"\line{\x\spa\x\spa\x\spb\x}%",
                r"\line{\x\spb\x\spa\x\spa\x}%",
                r"\line{\x\spa\x\spb\x\spb\x}%",
                r"\line{\x\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(90.0, Unit::Point),
                tolerance: 100,
                visual_incompatibility_demerits: 100,
                should_log: true,
            },
            9150,
        );
    }

    #[test]
    fn it_checks_multiple_visual_compatibilities() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}%",
            ],
            &[
                r"\def\a{\hbox to20pt{x}}%",
                r"\def\line#1{\hbox to410pt{#1}}%",
                r"\line{{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a}}%",
                r"\line{{\a} {\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a}}%",
                r"\line{{\a\a\a\a} {\a} {\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a}}%",
                r"\line{{\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a} {\a} {\a\a\a\a}}%",
                r"\line{{\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}}%",
                r"\line{{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a}}%",
                r"\line{{\a} {\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a}}%",
                r"\line{{\a\a\a\a} {\a} {\a\a\a}\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(410.0, Unit::Point),
                tolerance: 10000,
                visual_incompatibility_demerits: 60,
                should_log: true,
            },
            22025720,
        );
    }

    #[test]
    fn it_includes_penalties_in_demerits_calculation() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\setbox1=\hbox to15pt{x}%",
                r"\def\a{\copy1}%",
                r"{\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a}",
                r"{\a\a}{\penalty1000} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a} {\a\a} {\a\a} {\a}",
                r"{\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a}{\penalty-1000} ",
                r"{\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a}",
                r"{\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a}",
                r"{\a\a}{\penalty1000} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a}",
                r"{\a} {\a\a} {\a\a} {\a} {\a\a\a}%",
            ],
            &[
                r"\def\a{\hbox to15pt{x}}%",
                r"\def\line#1{\hbox to400pt{#1}}%",
                r"\line{{\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a}}%",
                r"\line{{\a\a}{\penalty1000} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a} {\a\a} {\a\a} {\a}}%",
                r"\line{{\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a}}%",
                r"\line{{\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a}}%",
                r"\line{{\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a}}%",
                r"\line{{\a\a}{\penalty1000} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a}}%",
                r"\line{{\a} {\a\a} {\a\a} {\a} {\a\a\a}\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(400.0, Unit::Point),
                tolerance: 1000,
                visual_incompatibility_demerits: 10000,
                should_log: true,
            },
            -177892,
        );
    }

    #[test]
    fn it_refuses_to_break_at_large_penalties() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\setbox1=\hbox to15pt{x}%",
                r"\def\a{\copy1}%",
                r"\def\nobreak{\penalty10000}%",
                r"{\a} {\a\a} {\a\a} {\a} {\a\a\a}",
                r"{\a\a\a}{\nobreak} {\a}{\nobreak} {\a\a}{\nobreak} {\a\a}{\nobreak} {\a}{\nobreak} {\a\a\a}{\nobreak}",
                r"{\a}{\nobreak} {\a\a}{\nobreak} {\a\a}{\nobreak} {\a}{\nobreak} {\a\a\a}{\nobreak} {\a\a\a}",
                r"{\a} {\a\a} {\a\a} {\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a}",
                r"{\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a} {\a\a}",
                r"{\a\a} {\a} {\a\a\a} {\a\a\a} {\a}%",
            ],
            &[
                r"\def\a{\hbox to15pt{x}}%",
                r"\def\line#1{\hbox to400pt{#1}}%",
                r"\def\nobreak{\penalty10000}%",
                r"\line{%",
                r"  {\a} {\a\a} {\a\a} {\a} {\a\a\a}{ }%",
                r"  {\a\a\a}{\nobreak} {\a}{\nobreak} {\a\a}{\nobreak} {\a\a}{\nobreak} {\a}{\nobreak} {\a\a\a}{\nobreak}{ }%",
                r"  {\a}{\nobreak} {\a\a}{\nobreak} {\a\a}{\nobreak} {\a}{\nobreak} {\a\a\a}{\nobreak} {\a\a\a}%",
                r"}%",
                r"\line{{\a} {\a\a} {\a\a} {\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a}}%",
                r"\line{{\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a} {\a\a}}%",
                r"\line{{\a\a} {\a} {\a\a\a} {\a\a\a} {\a}\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(400.0, Unit::Point),
                tolerance: 1000,
                visual_incompatibility_demerits: 10000,
                should_log: true,
            },
            12904,
        );
    }

    #[test]
    fn it_always_breaks_at_large_negative_penalties() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\setbox1=\hbox to15pt{x}%",
                r"\def\a{\copy1}%",
                r"\def\break{\penalty-10000}%",
                r"{\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a}{\break}",
                r"{\a\a} {\a\a} {\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a}",
                r"{\a\a} {\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a}{\break}",
                r"{\a} {\a\a} {\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a}{\break}",
                r"{\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a}%",
            ],
            &[
                r"\def\a{\hbox to15pt{x}}%",
                r"\def\line#1{\hbox to400pt{#1}}%",
                r"\def\break{\penalty-10000}%",
                r"\line{{\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a}}%",
                r"\line{{\a\a} {\a\a} {\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a} {\a\a}}%",
                r"\line{{\a\a} {\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a}}%",
                r"\line{{\a} {\a\a} {\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a}}%",
                r"\line{%",
                r"  {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a} {\a\a} {\a\a} {\a} {\a\a\a} {\a\a\a} {\a}%",
                r"  \penalty10000\hskip0pt plus1fil\penalty-10000%",
                r"}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(400.0, Unit::Point),
                tolerance: 1000,
                visual_incompatibility_demerits: 10000,
                should_log: true,
            },
            12100,
        );
    }

    #[test]
    fn it_adds_demerits_for_forced_breaks_only_if_other_potential_breaks_exist()
    {
        let paragraph = &[
            r"\def\a{\hbox to15pt{x}}%",
            r"\def\break{\penalty-10000}%",
            r"{\a}\break",
            r"{\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a}{\break}",
            r"{\a}%",
        ];

        expect_paragraph_to_parse_to_lines(
            paragraph,
            &[
                r"\def\a{\hbox to15pt{x}}%",
                r"\def\line#1{\hbox to375pt{#1}}%",
                r"\def\break{\penalty-10000}%",
                r"\line{{\a}}%",
                r"\line{{\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a}}%",
                r"\line{{\a}\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(375.0, Unit::Point),
                tolerance: 100,
                visual_incompatibility_demerits: 10000,
                should_log: true,
            },
            10225,
        );

        expect_paragraph_to_parse_to_lines(
            paragraph,
            &[
                r"\def\a{\hbox to15pt{x}}%",
                r"\def\line#1{\hbox to376pt{#1}}%",
                r"\def\break{\penalty-10000}%",
                r"\line{{\a}}%",
                r"\line{{\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a} {\a}}%",
                r"\line{{\a}\penalty10000\hskip0pt plus1fil\penalty-10000}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(376.0, Unit::Point),
                tolerance: 100,
                visual_incompatibility_demerits: 10000,
                should_log: true,
            },
            0,
        );
    }
}
