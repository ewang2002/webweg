use webweg::wrapper::input_types::{CourseLevelFilter, SearchRequestBuilder, SearchType};
use webweg::wrapper::wrapper_builder::WebRegWrapperBuilder;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let wrapper = WebRegWrapperBuilder::new()
        .with_cookies("my cookies here")
        .with_default_term("FA23")
        .try_build_wrapper()
        .unwrap();

    // Essentially registers the FA23 term with our session
    _ = wrapper.associate_term("FA23");

    let search = SearchRequestBuilder::new()
        .filter_courses_by(CourseLevelFilter::Graduate)
        .filter_courses_by(CourseLevelFilter::LowerDivision)
        .set_start_time(5, 30)
        .set_end_time(12 + 6, 30)
        .add_department("CSE")
        // This can be lowercase or uppercase
        .add_department("cogs")
        .add_department("MATH")
        .only_allow_open();

    let search_results = wrapper
        .default_request()
        .search_courses(SearchType::Advanced(search))
        .await;

    if let Ok(results) = search_results {
        for course in results {
            println!(
                "{} {}: {}",
                course.subj_code, course.course_code, course.course_title
            );
        }
    }
}
