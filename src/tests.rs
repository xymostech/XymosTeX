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
                "2877216447828127431Aa Gg Zz3276800Aa Gg Zz2752512Aa Gg Zz \n"
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

fn assert_tex_file_converts_to_dvi(
    tex_file_contents: &str,
    dvi_contents: &[u8],
) {
    let lines = tex_file_contents.split('\n').collect::<Vec<&str>>();

    let mut file_writer = DVIFileWriter::new();
    file_writer.start(
        (25400000, 473628672),
        1000,
        b"Made by XymosTeX".to_vec(),
    );

    with_parser(&lines[..], |parser| {
        let page = parser.parse_outer_vertical_box();
        file_writer.add_page(&page.list, &None, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    });

    file_writer.end();

    let test_file = file_writer.to_file();
    let test_pages = interpret_dvi_file(test_file);

    let real_file = DVIFile::new(dvi_contents).unwrap();
    let real_pages = interpret_dvi_file(real_file);

    for (test_page, real_page) in test_pages.iter().zip(real_pages.iter()) {
        for (key, val) in real_page.iter() {
            match test_page.get(key) {
                Some(_) => {}
                None => {
                    println!(
                        "Extra key/value in real page: {:?} {:?}",
                        key, val
                    );
                }
            }
        }

        for (key, val) in test_page.iter() {
            match real_page.get(key) {
                Some(_) => {}
                None => {
                    println!(
                        "Extra key/value in test page: {:?}, {:?}",
                        key, val
                    );
                }
            }
        }
    }

    assert_eq!(test_pages, real_pages);
}

#[test]
fn it_passes_stage_4_goals() {
    assert_tex_file_converts_to_dvi(
        include_str!("../examples/dvitest.tex"),
        include_bytes!("../examples/dvitest.dvi"),
    );
}

#[test]
fn it_passes_stage_5_goals() {
    assert_tex_file_converts_to_dvi(
        include_str!("../examples/math.tex"),
        include_bytes!("../examples/math.dvi"),
    );
}
