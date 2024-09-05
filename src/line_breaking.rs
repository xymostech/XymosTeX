use crate::boxes::{
    set_glue_for_dimen, BoxLayout, GlueSetResult, HorizontalBox, TeXBox,
};
use crate::dimension::Dimen;
use crate::glue::Glue;
use crate::list::HorizontalListElem;
use crate::state::{IntegerParameter, TeXState};

use std::collections::HashMap;

pub struct LineBreakingParams {
    pub hsize: Dimen,
    pub tolerance: i32,
}

#[derive(Debug, PartialEq)]
struct LineBreakingResult {
    total_demerits: u64,
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

#[derive(Debug, Clone)]
struct LineBreakBacktrace {
    prev_break: Option<usize>,
    total_demerits: u64,
}

#[derive(Debug)]
struct LineBreakGraph<'a> {
    // A list of places that can be broken at
    break_nodes: &'a Vec<LineBreakPoint>,
    // A list of backtraces from a given breakpoint to the best break before it.
    // Each value corresponds to an entry in break_nodes
    best_path_to: Vec<Option<LineBreakBacktrace>>,
}

impl<'a> LineBreakGraph<'a> {
    // Set up an empty line breaking graph given a list of indices.
    fn new_from_break_indices(break_indices: &'a Vec<LineBreakPoint>) -> Self {
        let mut graph = LineBreakGraph {
            break_nodes: break_indices,
            best_path_to: Vec::new(),
        };

        graph.best_path_to.push(Some(LineBreakBacktrace {
            prev_break: None,
            total_demerits: 0,
        }));
        graph.best_path_to.resize(2 + break_indices.len(), None);

        graph
    }

    fn index_of(&self, node: &LineBreakPoint) -> Option<usize> {
        self.break_nodes.iter().position(|n| n == node)
    }

    // Find the best demerits from the start to a given node, if one exists
    fn get_best_demerits_to_node(&self, to: &LineBreakPoint) -> Option<u64> {
        let to_index = self.index_of(to)?;
        if let Some(backtrace) = &self.best_path_to[to_index] {
            Some(backtrace.total_demerits)
        } else {
            None
        }
    }

    // Update the best path to a given node
    fn update_best_path_to_node(
        &mut self,
        to: &LineBreakPoint,
        from: &LineBreakPoint,
        demerits: u64,
    ) -> Option<()> {
        let to_index = self.index_of(to)?;
        let from_index = self.index_of(from)?;
        self.best_path_to[to_index] = Some(LineBreakBacktrace {
            prev_break: Some(from_index),
            total_demerits: demerits,
        });

        Some(())
    }

    // Return the best list of breaks to the end node
    fn get_best_breaks_to_end(&self) -> Option<LineBreakingResult> {
        let end_demerits =
            self.get_best_demerits_to_node(&LineBreakPoint::End)?;
        let mut all_breaks = vec![LineBreakPoint::End];
        let mut curr_break_backtrace = if let Some(backtrace) =
            &self.best_path_to[self.index_of(&LineBreakPoint::End)?]
        {
            backtrace
        } else {
            return None;
        };

        while let Some(prev_index) = curr_break_backtrace.prev_break {
            all_breaks.insert(0, self.break_nodes[prev_index]);
            curr_break_backtrace =
                if let Some(backtrace) = &self.best_path_to[prev_index] {
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
    Demerits { demerits: u64, badness: u64 },
}

fn get_demerits_for_line_between(
    list: &Vec<HorizontalListElem>,
    params: &LineBreakingParams,
    state: &TeXState,
    start: &LineBreakPoint,
    end: &LineBreakPoint,
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

    let line_penalty: u64 = 10;
    let penalty: i64 = 0;
    let demerits = if 0 <= penalty && penalty < 10000 {
        (line_penalty + badness).min(10000).pow(2) + (penalty.pow(2) as u64)
    } else if -10000 < penalty && penalty < 0 {
        (line_penalty + badness).min(10000).pow(2) - (penalty.pow(2) as u64)
    } else {
        (line_penalty + badness).min(10000).pow(2)
    };

    Some(DemeritResult::Demerits { demerits, badness })
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
    let mut graph = LineBreakGraph::new_from_break_indices(&line_breaks);

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

    // Whether we should log information about the line breaking procedure
    let should_log =
        state.get_integer_parameter(&IntegerParameter::TracingParagraphs) > 0;

    for line_break in line_breaks.iter().skip(1) {
        feasible_line_break_numbers
            .insert(*line_break, next_feasible_line_break_number);
        next_feasible_line_break_number += 1;

        let mut maybe_best_backwards_path: Option<LineBreakPoint> = None;
        let mut best_total_demerits: u64 = 0;
        for previous_break in reachable_previous_breaks.clone().iter() {
            let previous_demerits =
                graph.get_best_demerits_to_node(previous_break).unwrap();
            if let Some(demerits) = get_demerits_for_line_between(
                list,
                params,
                state,
                previous_break,
                line_break,
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
                            if should_log {
                                println!(
                                    "@ via @@{:?} b=* p=x d=*",
                                    feasible_line_break_numbers[previous_break]
                                );
                            }
                            maybe_best_backwards_path = Some(*previous_break);
                            // When this happens, even though this is a very bad
                            // situation, we add no demerits.
                            best_total_demerits = previous_demerits;
                        }
                    }
                    DemeritResult::TooLargeBadness => {} // ignore
                    DemeritResult::Demerits { demerits, badness } => {
                        if should_log {
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
            if should_log {
                // TODO(xymostech): Keep track of the line number of a given active
                // node to print here.
                println!(
                    "@@{:?}: line x.x t={} -> @@{:?}",
                    feasible_line_break_numbers[line_break],
                    best_total_demerits,
                    feasible_line_break_numbers[&best_backwards_path]
                );
            }
            reachable_previous_breaks.push(*line_break);
            graph.update_best_path_to_node(
                line_break,
                &best_backwards_path,
                best_total_demerits,
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
        expected_demerits: u64,
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
                        break;
                    }
                }

                assert_eq!(actual_boxes, expected_lines);
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
            },
            // NOTE: should be 22100 for first line and 10100 for last
            // line due to visual incompatibility, which hasn't been
            // implemented yet
            12100 + 12100 + 12100 + 12100 + 100,
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
                // NOTE: the commented lines are correct here. We choose the
                // incorrect breaks here because we don't take into account
                // visual incompatibility, which allows for multiple breaking
                // options at some points in the paragraph.
                //r"\line{{\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}}%",
                //r"\line{{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a}}%",
                //r"\line{{\a} {\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a}}%",
                r"\line{{\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a} {\a}}%",
                r"\line{{\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a}}%",
                r"\line{{\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a}}%",
                r"\line{{\a\a\a\a} {\a} {\a\a\a}\hskip0pt plus1fil}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(400.0, Unit::Point),
                tolerance: 10000,
            },
            // NOTE: should be 20000 more due to visual incompatibility, which
            // hasn't been implemented yet.
            100 + 324 + 656100 + 656100 + 656100 + 100 + 324 + 100,
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
            },
            100,
        );
    }

    #[test]
    fn it_splits_paragraphs_into_overfull_boxes_if_badness_is_low_enough() {
        expect_paragraph_to_parse_to_lines(
            &[
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\box\sp\box\sp\box\sp\box\sp\box%",
                r"\hskip0pt plus1fil%",
            ],
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
            },
            // The last 100 should be zero because this break is "forced".
            7333264 + 7333264 + 100,
        );

        expect_paragraph_to_parse_to_lines(
            &[
                r"\def\sp{\hskip 1pt plus3pt{}}%",
                r"\def\box{\hbox to50pt{a}}%",
                r"\box\sp\box\sp\box\sp\box\sp\box%",
                r"\hskip0pt plus1fil%",
            ],
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
            },
            // NOTE: should be 20000 higher due to visual incompatibility
            100000000 + 100000000 + 100,
        );
    }
}
