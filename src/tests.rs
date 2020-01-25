/// Integration tests to ensure that high-level expectations hold
use crate::testing::with_parser;

/// This test ensures that we pass the stage #2 goals.
#[test]
fn it_parses_horizontal_boxes() {
    with_parser(
        &[
            // Copied from examples/boxes.tex
            r"\def\boxcontents{Aa\hskip 5pt plus2pt minus2ptGg\hskip 3pt plus1fil minus1ptZz}%",
            r"\setbox1=\hbox{\boxcontents}%",
            r"\noindent\number\wd1 \number\ht1 \number\dp1 \box1%",
            r"\setbox1=\hbox to50pt{\boxcontents}%",
            r"\number\wd1 \box1%",
            r"\setbox1=\hbox to42pt{\boxcontents}%",
            r"\number\wd1 \box1%",
            r"\end",
        ],
        |parser| {
            let result: String =
                parser.parse_vertical_box_to_chars().into_iter().collect();

            assert_eq!(
                result,
                // This result is found by just running the same code through TeX.
                // We want to ensure that the dimensions are literally the exact
                // same as what TeX gives.
                "2877216447828127431Aa Gg Zz3276800Aa Gg Zz2752512Aa Gg Zz\n"
            );
        },
    );
}

/// This test ensures that we pass the stage #3 goals.
#[test]
fn it_parses_vertical_boxes() {
    with_parser(
        &[
            r"a\par",
            r"b\vskip1pt",
            r"\indent c\par",
            r"\noindent d\par",
            r"\hbox{e}",
            r"\setbox0=\vbox{",
            r"    \indent f\par",
            r"    g\vskip1pt",
            r"    \indent h\par",
            r"    \noindent i\par",
            r"    \hbox{j}",
            r"}",
            r"\noindent \number\ht0 \number\dp0 \par",
            r"\box0",
            r"\end",
        ],
        |parser| {
            let result: String =
                parser.parse_vertical_box_to_chars().into_iter().collect();

            assert_eq!(
                result,
                " a
 b
 c
d
e
3666375127431
 f
 g
 h
i
j

"
            );
        },
    );
}
