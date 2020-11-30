/// Integration tests to ensure that high-level expectations hold
use crate::box_to_dvi::DVIFileWriter;
use crate::dvi::{interpret_dvi_file, DVIFile};
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
            let result: String = parser
                .parse_outer_vertical_box()
                .to_chars()
                .into_iter()
                .collect();

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
            let result: String = parser
                .parse_outer_vertical_box()
                .to_chars()
                .into_iter()
                .collect();

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

/// This test ensure that we pass the stage #4 goals.
#[test]
fn it_matches_real_tex_output() {
    let dvitest_contents = include_str!("../examples/dvitest.tex");
    let lines = dvitest_contents.split('\n').collect::<Vec<&str>>();

    let mut file_writer = DVIFileWriter::new();
    file_writer.start(
        (25400000, 473628672),
        1000,
        "Made by XymosTeX".as_bytes().to_vec(),
    );

    with_parser(&lines[..], |parser| {
        let page = parser.parse_outer_vertical_box();
        file_writer.add_page(&page.list, &None, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    });

    file_writer.end();

    let test_file = file_writer.to_file();
    let test_pages = interpret_dvi_file(test_file);

    let real_dvi: &[u8] = include_bytes!("../examples/dvitest.dvi");
    let real_file = DVIFile::new(real_dvi).unwrap();
    let real_pages = interpret_dvi_file(real_file);

    assert_eq!(test_pages, real_pages);
}
