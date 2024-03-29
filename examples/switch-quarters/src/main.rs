use webweg::wrapper::WebRegWrapper;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let wrapper = WebRegWrapper::builder()
        .with_cookies("my cookies here")
        .try_build_wrapper()
        .unwrap();

    // Registers all active terms so we can switch between active quarters.
    _ = wrapper.register_all_terms().await;

    // Let's get all CSE 100 courses for FA23
    let cse100_fa23 = wrapper
        .req("WI24")
        .parsed()
        .get_course_info("CSE", "100")
        .await;

    match cse100_fa23 {
        Ok(courses) => {
            for course in courses {
                println!("{course}")
            }
        }
        Err(err) => {
            eprintln!("{err}");
        }
    }

    // But we can also switch to another active quarter
    let cse100_s223 = wrapper
        .req("S223")
        .parsed()
        .get_course_info("CSE", "100")
        .await;

    match cse100_s223 {
        Ok(courses) => {
            for course in courses {
                println!("{course}")
            }
        }
        Err(err) => {
            eprintln!("{err}");
        }
    }

    // You can also register terms that are probably hidden (it's available on WebReg,
    // but is hidden from the get_term API endpoint)
    _ = wrapper.associate_term("WI24").await;

    // Of course, we can get more than just course info when switching quarters.
    let cse_course_notes = wrapper
        .req("WI24")
        .parsed()
        .get_section_notes_by_course("CSE", "290")
        .await
        .unwrap();
    println!("{cse_course_notes:?}");
}
