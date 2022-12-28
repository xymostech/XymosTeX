use crate::boxes::{set_glue_for_dimen, BoxLayout, HorizontalBox, TeXBox};
use crate::dimension::{Dimen, Unit};
use crate::glue::Glue;
use crate::list::HorizontalListElem;
use crate::state::TeXState;

use std::collections::{HashMap, HashSet};

pub struct LineBreakingParams {
    hsize: Dimen,
}

#[derive(Debug, PartialEq)]
struct LineBreakingResult {
    total_demerits: u64,
    all_breaks: Vec<LineBreakPoint>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum LineBreakPoint {
    Start,
    End,
    BreakAtIndex(usize),
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
struct LineBreakGraph {
    break_nodes: Vec<LineBreakPoint>,
    best_path_to: Vec<Option<LineBreakBacktrace>>,
    line_edges: HashMap<usize, HashSet<usize>>,
    line_backwards_edges: HashMap<usize, HashSet<usize>>,
}

impl LineBreakGraph {
    fn new_from_break_indices(break_indices: &Vec<usize>) -> Self {
        let mut graph = LineBreakGraph {
            break_nodes: Vec::new(),
            best_path_to: Vec::new(),
            line_edges: HashMap::new(),
            line_backwards_edges: HashMap::new(),
        };

        graph.break_nodes.push(LineBreakPoint::Start);
        graph.best_path_to.push(Some(LineBreakBacktrace {
            prev_break: None,
            total_demerits: 0,
        }));
        graph.break_nodes.append(
            &mut break_indices
                .iter()
                .map(|index| LineBreakPoint::BreakAtIndex(*index))
                .collect(),
        );
        graph.break_nodes.push(LineBreakPoint::End);
        graph.best_path_to.resize(2 + break_indices.len(), None);

        graph.add_edge(0, graph.break_nodes.len() - 1);

        for (ii, start_index) in break_indices.iter().enumerate() {
            let i = ii + 1;

            graph.add_edge(0, i);
            graph.add_edge(i, graph.break_nodes.len() - 1);

            for (jj, end_index) in break_indices.iter().enumerate() {
                let j = jj + 1;

                if start_index < end_index {
                    graph.add_edge(i, j);
                }
            }
        }

        graph
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        (*self.line_edges.entry(from).or_insert(HashSet::new())).insert(to);
        (*self
            .line_backwards_edges
            .entry(to)
            .or_insert(HashSet::new()))
        .insert(from);
    }

    fn index_of(&self, node: &LineBreakPoint) -> Option<usize> {
        self.break_nodes.iter().position(|n| n == node)
    }

    fn get_nodes_connected_to(
        &self,
        from: &LineBreakPoint,
    ) -> Vec<LineBreakPoint> {
        if let Some(from_index) = self.index_of(from) {
            if let Some(to_indices) = self.line_edges.get(&from_index) {
                to_indices
                    .iter()
                    .map(|to_index| self.break_nodes[*to_index])
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    fn get_nodes_connecting_to(
        &self,
        to: &LineBreakPoint,
    ) -> Vec<LineBreakPoint> {
        if let Some(to_index) = self.index_of(to) {
            if let Some(from_indices) = self.line_backwards_edges.get(&to_index)
            {
                from_indices
                    .iter()
                    .map(|from_index| self.break_nodes[*from_index])
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    fn get_best_demerits_to_node(&self, to: &LineBreakPoint) -> Option<u64> {
        let to_index = self.index_of(to)?;
        if let Some(backtrace) = &self.best_path_to[to_index] {
            Some(backtrace.total_demerits)
        } else {
            None
        }
    }

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

fn get_available_break_indices(list: &Vec<HorizontalListElem>) -> Vec<usize> {
    let mut available_break_indices = Vec::new();

    for (i, curr) in list.iter().enumerate() {
        match curr {
            HorizontalListElem::HSkip(_) => {
                available_break_indices.push(i);
            }
            _ => (),
        }
    }

    available_break_indices
}

fn get_demerits_for_line_between(
    list: &Vec<HorizontalListElem>,
    params: &LineBreakingParams,
    state: &TeXState,
    start: &LineBreakPoint,
    end: &LineBreakPoint,
) -> Option<u64> {
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
    let badness = glue_set.get_badness();

    let line_penalty: u64 = 10;
    let penalty: i64 = 0;
    let demerits = if 0 <= penalty && penalty < 10000 {
        (line_penalty + badness).pow(2) + (penalty.pow(2) as u64)
    } else if -10000 < penalty && penalty < 0 {
        (line_penalty + badness).pow(2) - (penalty.pow(2) as u64)
    } else {
        (line_penalty + badness).pow(2)
    };

    Some(demerits)
}

fn generate_best_list_break_option_with_params(
    list: &Vec<HorizontalListElem>,
    params: &LineBreakingParams,
    state: &TeXState,
) -> Option<LineBreakingResult> {
    let break_indices = get_available_break_indices(&list);
    let mut graph = LineBreakGraph::new_from_break_indices(&break_indices);

    let mut seen_nodes = HashSet::new();
    let mut nodes_to_search = vec![LineBreakPoint::Start];

    while let Some(node) = nodes_to_search.pop() {
        if seen_nodes.contains(&node) {
            continue;
        }

        let from_nodes = graph
            .get_nodes_connecting_to(&node)
            .into_iter()
            .collect::<HashSet<_>>();
        if !from_nodes.is_subset(&seen_nodes) {
            continue;
        }

        seen_nodes.insert(node);
        let node_demerits =
            if let Some(demerits) = graph.get_best_demerits_to_node(&node) {
                demerits
            } else {
                continue;
            };

        for connected_node in graph.get_nodes_connected_to(&node).iter() {
            if let Some(demerits) = get_demerits_for_line_between(
                &list,
                &params,
                state,
                &node,
                &connected_node,
            ) {
                let total_demerits = node_demerits + demerits;
                if let Some(prev_best_demerits) =
                    graph.get_best_demerits_to_node(connected_node)
                {
                    if total_demerits < prev_best_demerits {
                        graph.update_best_path_to_node(
                            connected_node,
                            &node,
                            total_demerits,
                        );
                    }
                } else {
                    graph.update_best_path_to_node(
                        connected_node,
                        &node,
                        total_demerits,
                    );
                }

                nodes_to_search.push(*connected_node);
            }
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
                r"\line{{\a} {\a\a\a}  {\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a}}%",
                r"\line{{\a\a\a\a} {\a} {\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a}}%",
                r"\line{{\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a} {\a} {\a\a\a\a}}%",
                r"\line{{\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a} {\a} {\a\a\a}}%",
                r"\line{{\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a} {\a\a\a\a}}%",
                r"\line{{\a} {\a\a\a} {\a} {\a\a\a\a} {\a\a} {\a\a\a} {\a\a\a} {\a\a}}%",
                r"\line{{\a\a\a\a} {\a} {\a\a\a}\hskip0pt plus1fil}%",
            ],
            LineBreakingParams {
                hsize: Dimen::from_unit(400.0, Unit::Point),
            },
            // NOTE: should be 20000 more due to visual incompatibility, which
            // hasn't been implemented yet.
            100 + 324 + 656100 + 656100 + 656100 + 100 + 324 + 100,
        );
    }
}
