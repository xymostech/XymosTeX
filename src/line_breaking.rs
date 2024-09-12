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
    list: &Vec<HorizontalListElem>,
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

#[derive(Debug, Clone, Copy)]
enum VisualClassification {
    VeryLoose = 0,
    Loose = 1,
    Decent = 2,
    Tight = 3,
}

impl VisualClassification {
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

#[derive(Debug, Clone)]
struct LineBreakBacktrace {
    prev_break: Option<LineBreakPoint>,
    total_demerits: i64,
    prev_line_classification: VisualClassification,
}

#[derive(Debug)]
struct LineBreakGraph {
    // A list of backtraces from a given breakpoint to the best break before it.
    // Each value corresponds to an entry in break_nodes
    best_path_to: HashMap<LineBreakPoint, LineBreakBacktrace>,
}

impl LineBreakGraph {
    // Set up an empty line breaking graph given a list of indices.
    fn new() -> Self {
        let mut graph = LineBreakGraph {
            best_path_to: HashMap::new(),
        };

        graph.best_path_to.insert(
            LineBreakPoint::Start,
            LineBreakBacktrace {
                prev_break: None,
                total_demerits: 0,
                prev_line_classification: VisualClassification::Decent,
            },
        );

        graph
    }

    // Find the best demerits from the start to a given node, if one exists
    fn get_best_demerits_to_node(&self, to: &LineBreakPoint) -> Option<i64> {
        self.best_path_to
            .get(to)
            .map(|backtrace| backtrace.total_demerits)
    }

    fn get_classification_of_best_line_before_node(
        &self,
        to: &LineBreakPoint,
    ) -> Option<VisualClassification> {
        self.best_path_to
            .get(to)
            .map(|backtrace| backtrace.prev_line_classification)
    }

    // Update the best path to a given node
    fn update_best_path_to_node(
        &mut self,
        to: &LineBreakPoint,
        from: &LineBreakPoint,
        demerits: i64,
        prev_line_classification: VisualClassification,
    ) {
        self.best_path_to.insert(
            *to,
            LineBreakBacktrace {
                prev_break: Some(*from),
                total_demerits: demerits,
                prev_line_classification,
            },
        );
    }

    // Return the best list of breaks to the end node
    fn get_best_breaks_to_end(&self) -> Option<LineBreakingResult> {
        let end_demerits =
            self.get_best_demerits_to_node(&LineBreakPoint::End)?;
        let mut all_breaks = vec![LineBreakPoint::End];
        let mut curr_break_backtrace = if let Some(backtrace) =
            self.best_path_to.get(&LineBreakPoint::End)
        {
            backtrace
        } else {
            return None;
        };

        while let Some(prev_break) = curr_break_backtrace.prev_break {
            all_breaks.insert(0, prev_break);
            curr_break_backtrace =
                if let Some(backtrace) = &self.best_path_to.get(&prev_break) {
                    backtrace
                } else {
                    return None;
                };
        }

        Some(LineBreakingResult {
            total_demerits: end_demerits,
            all_breaks: all_breaks,
        })
    }
}

fn get_available_break_indices(
    list: &Vec<HorizontalListElem>,
) -> Vec<LineBreakPoint> {
    let mut available_break_indices = Vec::new();

    available_break_indices.push(LineBreakPoint::Start);
    for (i, curr) in list.iter().enumerate() {
        match curr {
            HorizontalListElem::HSkip(_) => {
                available_break_indices.push(LineBreakPoint::BreakAtIndex(i));
            }
            _ => (),
        }
    }
    available_break_indices.push(LineBreakPoint::End);

    available_break_indices
}

#[derive(Debug)]
enum DemeritResult {
    Overfull,
    TooLargeBadness,
    Demerits {
        demerits: i64,
        badness: u64,
        classification: VisualClassification,
    },
}

fn get_demerits_for_line_between(
    list: &Vec<HorizontalListElem>,
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

    if badness > params.tolerance as u64 {
        return Some(DemeritResult::TooLargeBadness);
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
    let penalty: i64 = 0;
    let base_demerits = if 0 <= penalty && penalty < 10000 {
        (line_penalty + badness as i64).min(10000).pow(2) + penalty.pow(2)
    } else if -10000 < penalty && penalty < 0 {
        (line_penalty + badness as i64).min(10000).pow(2) - penalty.pow(2)
    } else {
        (line_penalty + badness as i64).min(10000).pow(2)
    };

    Some(DemeritResult::Demerits {
        demerits: base_demerits + additional_demerits,
        badness,
        classification: visual_classification,
    })
}

// Given a horizontal list, try to generate the best line breaks which match the
// line breaking params.
fn generate_best_list_break_option_with_params(
    list: &Vec<HorizontalListElem>,
    params: &LineBreakingParams,
    state: &TeXState,
) -> Option<LineBreakingResult> {
    // This function implements the Knuth-Plass line breaking algorithm. This is
    // an optimized version of a shortest path graph search, where each
    // available break point is a node and the weight of the edges between them
    // is the badness of setting the line between those break points.

    let line_breaks = get_available_break_indices(&list);
    let mut graph = LineBreakGraph::new();

    // Keep track of previous breakpoints that we've looked at already, that are
    // still reachable from the current break without being overfull.
    let mut reachable_previous_breaks: Vec<LineBreakPoint> =
        Vec::from([LineBreakPoint::Start]);

    // For logging, we don't want to refer to our `LineBreakPoint`s using our
    // internal representation, so we sequentially number the feasible
    // breakpoints we find, with the start referring to 0.
    let mut next_feasible_line_break_number = 1;
    let mut feasible_line_break_numbers: HashMap<LineBreakPoint, usize> =
        HashMap::new();
    feasible_line_break_numbers.insert(LineBreakPoint::Start, 0);

    for line_break in line_breaks.iter().skip(1) {
        let mut maybe_best_backwards_path: Option<LineBreakPoint> = None;
        let mut best_classification: Option<VisualClassification> = None;
        let mut best_total_demerits: i64 = 0;
        for previous_break in reachable_previous_breaks.clone().iter() {
            let previous_demerits =
                graph.get_best_demerits_to_node(previous_break).unwrap();
            let previous_classification = graph
                .get_classification_of_best_line_before_node(previous_break);
            if let Some(demerits) = get_demerits_for_line_between(
                list,
                params,
                state,
                previous_break,
                line_break,
                previous_classification,
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

                        if reachable_previous_breaks.len() == 0 {
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
                                    "@ via @@{:?} b=* p=x d=*",
                                    feasible_line_break_numbers[previous_break]
                                );
                            }
                            maybe_best_backwards_path = Some(*previous_break);
                            best_classification =
                                Some(VisualClassification::Tight);
                            // When this happens, even though this is a very bad
                            // situation, we add no demerits.
                            best_total_demerits = previous_demerits;
                        }
                    }
                    DemeritResult::TooLargeBadness => {} // ignore
                    DemeritResult::Demerits {
                        demerits,
                        badness,
                        classification,
                    } => {
                        if params.should_log {
                            println!(
                                "@ via @@{:?} b={} p=x d={}",
                                feasible_line_break_numbers[previous_break],
                                badness,
                                demerits
                            );
                        }
                        if maybe_best_backwards_path.is_none()
                            || demerits + previous_demerits
                                <= best_total_demerits
                        {
                            maybe_best_backwards_path = Some(*previous_break);
                            best_classification = Some(classification);
                            best_total_demerits = demerits + previous_demerits;
                        }
                    }
                }
            } else {
                // This branch can happen when we try to break at the \hskip
                // inserted right before the end of the paragraph.
                // TODO(xymostech): Stop trying to break here. TeX normally
                // inserts a \nobreak before that \hskip to prevent this.
            }
        }

        if let Some(best_backwards_path) = maybe_best_backwards_path {
            feasible_line_break_numbers
                .insert(*line_break, next_feasible_line_break_number);
            next_feasible_line_break_number += 1;

            if params.should_log {
                // TODO(xymostech): Keep track of the line number of a given active
                // node to print here.
                println!(
                    "@@{:?}: line x.{} t={} -> @@{:?}",
                    feasible_line_break_numbers[line_break],
                    best_classification.unwrap() as u8,
                    best_total_demerits,
                    feasible_line_break_numbers[&best_backwards_path]
                );
            }
            reachable_previous_breaks.push(*line_break);
            graph.update_best_path_to_node(
                line_break,
                &best_backwards_path,
                best_total_demerits,
                best_classification.unwrap(),
            );
        }
    }

    graph.get_best_breaks_to_end()
}

pub fn break_horizontal_list_to_lines_with_params(
    list: &Vec<HorizontalListElem>,
    params: LineBreakingParams,
    state: &TeXState,
) -> Option<Vec<TeXBox>> {
    let best_option =
        generate_best_list_break_option_with_params(&list, &params, state)?;

    let break_pairs = best_option
        .all_breaks
        .iter()
        .zip(best_option.all_breaks.iter().skip(1));
    let line_boxes = break_pairs
        .map(|(start, end)| {
            let (start_index, end_index) =
                get_list_indices_for_breaks(list, &start, &end).unwrap();
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

    use crate::dimension::Unit;
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
                let hlist = parser.parse_horizontal_list(false, false);

                let best_break = generate_best_list_break_option_with_params(
                    &hlist,
                    &params,
                    parser.state,
                )
                .unwrap();

                assert_eq!(best_break.total_demerits, expected_demerits);

                let actual_boxes = break_horizontal_list_to_lines_with_params(
                    &hlist,
                    params,
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
                            "{:?} elems vs {:?} elems",
                            actual_box.to_chars(),
                            expected_line.to_chars()
                        );
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
                r"\hskip0pt plus1fil%",
            ],
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"\hbox to150pt{{\a} {\a\a\a\a} {\a\a}\hskip0pt plus1fil}%",
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
                r"\hskip0pt plus1fil%",
            ],
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"\hbox to105pt{{\a} {\a\a\a\a}}%",
                r"\hbox to105pt{{\a\a}\hskip0pt plus1fil}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(105.0, Unit::Point),
                tolerance: 10000,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            12100 + 100,
        );
    }

    #[test]
    fn test_whole_paragraph_splitting() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}%",
                r"\hskip0pt plus1fil%",
            ],
            &[
                r"\setbox1=\hbox to20pt{x}%",
                r"\def\a{\copy1}%",
                r"\def\line#1{\hbox to105pt{#1}}%",
                r"\line{{\a} \a\a\a\a}%",
                r"\line{{\a\a} \a\a\a}%",
                r"\line{{\a\a\a} \a\a}%",
                r"\line{{\a\a\a\a} \a}%",
                r"\line{{\a\a\a}\hskip0pt plus1fil}%",
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
                r"\hskip0pt plus1fil%",
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
                r"\line{{\a\a\a\a} {\a} {\a\a\a}\hskip0pt plus1fil}%",
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
            &[
                r"\hbox to90pt{ab\hskip0pt plus1fil cd} %",
                r"efg hij%",
                r"\hskip0pt plus1fil%",
            ],
            &[
                r"\def\line#1{\hbox to80pt{#1}}%",
                r"\line{\hbox to90pt{ab\hskip0pt plus1fil cd}}%",
                r"\line{efg hij\hskip0pt plus1fil}%",
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
            r"\hskip0pt plus1fil%",
        ];

        expect_paragraph_to_parse_to_lines(
            &paragraph,
            &[
                r"\def\line#1{\hbox to110pt{#1}}%",
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\line{\box\sp\box}%",
                r"\line{\box\sp\box}%",
                r"\line{\box\hskip0pt plus1fil}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(110.0, Unit::Point),
                tolerance: 2700,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            // The last 100 should be zero because this break is "forced".
            7333264 + 7333264 + 100,
        );

        expect_paragraph_to_parse_to_lines(
            &paragraph,
            &[
                r"\def\line#1{\hbox to110pt{#1}}%",
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\line{\box\sp\box\sp\box}%",
                r"\line{\box\sp\box\hskip0pt plus1fil}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(110.0, Unit::Point),
                tolerance: 2600,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            100,
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
                r"\hskip0pt plus1fil%",
            ],
            &[
                r"\def\line#1{\hbox to120pt{#1}}%",
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\line{\box\sp\box\sp\box}%",
                r"\line{\box\sp\box\hskip0pt plus1fil}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(120.0, Unit::Point),
                tolerance: 9999,
                visual_incompatibility_demerits: 0,
                should_log: true,
            },
            // This should actually be zero, because the last break is "forced"
            // and in this case we don't add any demerits (in this case we're
            // adding \linepenalty).
            100,
        );

        expect_paragraph_to_parse_to_lines(
            &[
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                // Even though this needs to stretch a huge amount, the
                // tolerance is infinite so this is allowed
                r"\box\sp\box\sp\box\sp\box\sp\box%",
                r"\hskip0pt plus1fil%",
            ],
            &[
                r"\def\line#1{\hbox to120pt{#1}}%",
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\line{\box\sp\box}%",
                r"\line{\box\sp\box}%",
                r"\line{\box\hskip0pt plus1fil}%",
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
            r"\hskip0pt plus1fil%",
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
                r"\line{\x\spb\x\spb\x\hskip0pt plus1fil}%",
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
                r"\line{\x\hskip0pt plus1fil}%",
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
}
