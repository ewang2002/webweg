use std::collections::HashMap;
use std::time::Duration;

use reqwest::header::{COOKIE, USER_AGENT};
use reqwest::{Client, RequestBuilder};
use url::Url;

use crate::raw_types::{
    RawDepartmentElement, RawEvent, RawPrerequisite, RawScheduledMeeting, RawSubjectElement,
    RawWebRegMeeting, RawWebRegSearchResultItem,
};
use crate::types::{
    CourseSection, EnrollWaitAdd, Event, EventAdd, GradeOption, PlanAdd, PrerequisiteInfo,
    ScheduledSection, WrapperError,
};
use crate::wrapper::constants::{
    ALL_SCHEDULE, CHANGE_ENROLL, COURSE_DATA, CURR_SCHEDULE, DEFAULT_SCHEDULE_NAME, DEPT_LIST,
    ENROLL_ADD, ENROLL_DROP, ENROLL_EDIT, EVENT_ADD, EVENT_EDIT, EVENT_GET, EVENT_REMOVE, PLAN_ADD,
    PLAN_EDIT, PLAN_REMOVE, PLAN_REMOVE_ALL, PREREQS_INFO, REMOVE_SCHEDULE, RENAME_SCHEDULE,
    SEND_EMAIL, SUBJ_LIST, WAILIST_DROP, WAITLIST_ADD, WAITLIST_EDIT,
};
use crate::wrapper::search::{DayOfWeek, SearchType};
use crate::wrapper::ww_helper::{process_get_result, process_post_response};
use crate::wrapper::ww_parser::{
    build_search_course_url, parse_course_info, parse_enrollment_count, parse_get_events,
    parse_prerequisites, parse_schedule,
};
use crate::wrapper::WebRegWrapper;
use crate::{types, util};

pub struct WrapperTermRequestBuilder<'a> {
    cookies: &'a str,
    client: &'a Client,
    term: &'a str,
    user_agent: &'a str,
    timeout: Duration,
}

impl<'a> WrapperTermRequestBuilder<'a> {
    pub fn new_request(wrapper: &'a WebRegWrapper) -> Self {
        Self {
            cookies: &wrapper.cookies,
            client: &wrapper.client,
            term: &wrapper.term,
            user_agent: &wrapper.user_agent,
            timeout: wrapper.default_timeout,
        }
    }

    pub fn override_cookies(mut self, cookies: &'a str) -> Self {
        self.cookies = cookies;
        self
    }

    pub fn override_client(mut self, client: &'a Client) -> Self {
        self.client = client;
        self
    }

    pub fn override_term(mut self, term: &'a str) -> Self {
        self.term = term;
        self
    }

    pub fn override_user_agent(mut self, user_agent: &'a str) -> Self {
        self.user_agent = user_agent;
        self
    }

    pub fn override_timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    pub fn finish_building(self) -> WrapperTermRequest<'a> {
        WrapperTermRequest { info: self }
    }
}

pub struct WrapperTermRequest<'a> {
    info: WrapperTermRequestBuilder<'a>,
}

impl<'a> WrapperTermRequest<'a> {
    /// Gets all prerequisites for a specified course for the term set by the wrapper.
    ///
    /// # Parameters
    /// - `subject_code`: The subject code. For example, if you wanted to check `MATH 100B`, you
    /// would put `MATH`.
    /// - `course_code`: The course code. For example, if you wanted to check `MATH 100B`, you
    /// would put `100B`.
    ///
    /// # Returns
    /// All prerequisites for the specified course. This is a structure that has two fields: one
    /// for all exam prerequisites, and one for all course prerequisites.
    ///
    ///
    /// ### Course Prerequisites
    ///
    /// This is a vector of vector of prerequisites, where each vector contains one or
    /// more prerequisites. Any prerequisites in the same vector means that you only need
    /// one of those prerequisites to fulfill that requirement.
    ///
    /// For example, if this value was `[[a, b], [c, d, e], [f]], then this means
    /// that you need
    /// - one of 'a' or 'b', *and*
    /// - one of 'c', 'd', or 'e', *and*
    /// - f.
    ///
    ///
    /// ### Exam Prerequisites
    /// Exam prerequisites will satisfy all of the requirements defined by course prerequisites.
    /// In other words, if you satisfy one of the exam prerequisites, you automatically satisfy
    /// all of the course prerequisites.
    ///
    /// # Example
    /// ```rust,no_run
    /// use webweg::wrapper::WebRegWrapper;
    /// use webweg::wrapper::wrapper_builder::WebRegWrapperBuilder;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapperBuilder::new()
    ///     .with_cookies("your cookies here")
    ///     .with_default_term("FA23")
    ///     .try_build_wrapper()
    ///     .unwrap();
    ///
    /// let prereqs = wrapper
    ///     .default_request()
    ///     .get_prerequisites("COGS", "108")
    ///     .await;
    ///
    /// if let Ok(prereq_info) = prereqs {
    ///     println!("{:?}", prereq_info.course_prerequisites);
    ///     println!("{:?}", prereq_info.exam_prerequisites);
    /// }
    /// # }
    /// ```
    pub async fn get_prerequisites(
        &self,
        subject_code: impl AsRef<str>,
        course_code: impl AsRef<str>,
    ) -> types::Result<PrerequisiteInfo> {
        let crsc_code = util::get_formatted_course_num(course_code.as_ref());
        let url = Url::parse_with_params(
            PREREQS_INFO,
            &[
                ("subjcode", subject_code.as_ref()),
                ("crsecode", crsc_code.as_str()),
                ("termcode", self.info.term),
                ("_", util::get_epoch_time().to_string().as_ref()),
            ],
        )?;

        parse_prerequisites(
            process_get_result::<Vec<RawPrerequisite>>(
                self.info
                    .client
                    .get(url)
                    .header(COOKIE, self.info.cookies)
                    .header(USER_AGENT, self.info.user_agent)
                    .send()
                    .await,
            )
            .await?,
        )
    }

    /// Gets your current schedule.
    ///
    /// # Parameters
    /// - `schedule_name`: The schedule that you want to get. If `None` is given, this will default
    /// to your main schedule.
    ///
    /// # Returns
    /// Either a vector of sections that appear in your schedule, or an error message if something
    /// went wrong.
    ///
    /// # Examples
    ///
    /// Getting the default schedule.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// // Pass in "None" for the default.
    /// let schedule = wrapper.get_schedule(None).await;
    /// match schedule {
    ///     Ok(s) => s.iter().for_each(|sec| println!("{}", sec.to_string())),
    ///     Err(e) => eprintln!("An error occurred! {}", e)
    /// }
    ///
    /// # }
    /// ```
    ///
    /// Getting the schedule with name "`Other Schedule`."
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// // Pass in "None" for the default.
    /// let schedule = wrapper.get_schedule(Some("Other Schedule")).await;
    /// match schedule {
    ///     Ok(s) => s.iter().for_each(|sec| println!("{}", sec.to_string())),
    ///     Err(e) => eprintln!("An error occurred! {}", e)
    /// }
    /// # }
    /// ```
    pub async fn get_schedule(
        &self,
        schedule_name: Option<&str>,
    ) -> types::Result<Vec<ScheduledSection>> {
        let url = Url::parse_with_params(
            CURR_SCHEDULE,
            &[
                ("schedname", schedule_name.unwrap_or(DEFAULT_SCHEDULE_NAME)),
                ("final", ""),
                ("sectnum", ""),
                ("termcode", self.info.term),
                ("_", util::get_epoch_time().to_string().as_str()),
            ],
        )?;

        parse_schedule(
            process_get_result::<Vec<RawScheduledMeeting>>(
                self.info
                    .client
                    .get(url)
                    .header(COOKIE, self.info.cookies)
                    .header(USER_AGENT, self.info.user_agent)
                    .send()
                    .await,
            )
            .await?,
        )
    }

    /// Gets enrollment count for a particular course.
    ///
    /// Unlike the `get_course_info` function, this function only returns a vector of sections
    /// with the proper enrollment counts. Therefore, the `meetings` vector will always be
    /// empty as it is not relevant.
    ///
    /// Additionally, this function only returns one of some number of possible instructors.
    ///
    /// If you want full course information, use `get_course_info`. If you only care about the
    /// number of people enrolled in a section, this function is for you.
    ///
    /// # Parameters
    /// - `subject_code`: The subject code. For example, if you wanted to check `MATH 100B`, you
    /// would put `MATH`.
    /// - `course_num`: The course number. For example, if you wanted to check `MATH 100B`, you
    /// would put `100B`.
    ///
    /// # Returns
    /// Either a vector with all sections that match the given subject code & course code, or an
    /// error if one occurred.
    ///
    /// # Example
    /// Suppose we wanted to find all sections of CSE 101 for the sole purpose of seeing how
    /// many people are enrolled.
    /// ```rust,no_run
    /// use reqwest::Client;
    ///
    /// use webweg::wrapper::WebRegWrapper;
    ///
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// let sections = wrapper.get_enrollment_count("CSE", "101").await;
    /// match sections {
    ///     Ok(s) => s.iter().for_each(|sec| println!("{}", sec.to_string())),
    ///     Err(e) => eprintln!("An error occurred! {}", e)
    /// }
    /// # }
    /// ```
    pub async fn get_enrollment_count(
        &self,
        subject_code: impl AsRef<str>,
        course_num: impl AsRef<str>,
    ) -> types::Result<Vec<CourseSection>> {
        let crsc_code = util::get_formatted_course_num(course_num.as_ref());
        let url = Url::parse_with_params(
            COURSE_DATA,
            &[
                ("subjcode", subject_code.as_ref()),
                ("crsecode", crsc_code.as_str()),
                ("termcode", self.info.term),
                ("_", util::get_epoch_time().to_string().as_ref()),
            ],
        )?;

        let course_dept_id = format!(
            "{} {}",
            subject_code.as_ref().trim(),
            course_num.as_ref().trim()
        )
        .to_uppercase();

        parse_enrollment_count(
            process_get_result::<Vec<RawWebRegMeeting>>(self.init_get_request(url).send().await)
                .await?,
            course_dept_id,
        )
    }

    /// Gets course information for a particular course.
    ///
    /// Note that WebReg provides this information in a way that makes it hard to use; in
    /// particular, WebReg separates each lecture, discussion, final exam, etc. from each other.
    /// This function attempts to figure out which lecture/discussion/final exam/etc. correspond
    /// to which section.
    ///
    /// # Parameters
    /// - `subject_code`: The subject code. For example, if you wanted to check `MATH 100B`, you
    /// would put `MATH`.
    /// - `course_num`: The course number. For example, if you wanted to check `MATH 100B`, you
    /// would put `100B`.
    ///
    /// # Returns
    /// A result containing either:
    /// - A vector with all possible sections that match the given subject code & course code.
    /// - Or the error that occurred.
    ///
    /// # Example
    /// Let's suppose we wanted to find all sections of CSE 105. This is how we would do this.
    /// Note that this will contain a lot of information.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// let sections = wrapper.get_course_info("CSE", "105").await;
    /// match sections {
    ///     Ok(s) => s.iter().for_each(|sec| println!("{}", sec.to_string())),
    ///     Err(e) => eprintln!("An error occurred! {}", e)
    /// }
    /// # }
    /// ```
    pub async fn get_course_info(
        &self,
        subject_code: impl AsRef<str>,
        course_num: impl AsRef<str>,
    ) -> types::Result<Vec<CourseSection>> {
        let crsc_code = util::get_formatted_course_num(course_num.as_ref());
        let course_dept_id = format!(
            "{} {}",
            subject_code.as_ref().trim(),
            course_num.as_ref().trim()
        )
        .to_uppercase();

        let url = self.init_get_request(Url::parse_with_params(
            COURSE_DATA,
            &[
                ("subjcode", subject_code.as_ref()),
                ("crsecode", crsc_code.as_str()),
                ("termcode", self.info.term),
                ("_", util::get_epoch_time().to_string().as_ref()),
            ],
        )?);

        parse_course_info(
            process_get_result::<Vec<RawWebRegMeeting>>(url.send().await).await?,
            course_dept_id,
        )
    }

    /// Gets a list of all departments that are offering courses for this term.
    ///
    /// # Returns
    /// A vector of department codes.
    pub async fn get_department_codes(&self) -> types::Result<Vec<String>> {
        Ok(process_get_result::<Vec<RawDepartmentElement>>(
            self.init_get_request(Url::parse_with_params(
                DEPT_LIST,
                &[
                    ("termcode", self.info.term),
                    ("_", util::get_epoch_time().to_string().as_str()),
                ],
            )?)
            .send()
            .await,
        )
        .await?
        .into_iter()
        .map(|x| x.dep_code.trim().to_string())
        .collect::<Vec<_>>())
    }

    /// Gets a list of all subjects that have at least one course offered for this term.
    ///
    /// # Returns
    /// A vector of subject codes.
    pub async fn get_subject_codes(&self) -> types::Result<Vec<String>> {
        Ok(process_get_result::<Vec<RawSubjectElement>>(
            self.init_get_request(Url::parse_with_params(
                SUBJ_LIST,
                &[
                    ("termcode", self.info.term),
                    ("_", util::get_epoch_time().to_string().as_str()),
                ],
            )?)
            .send()
            .await,
        )
        .await?
        .into_iter()
        .map(|x| x.subject_code.trim().to_string())
        .collect::<Vec<_>>())
    }

    /// Gets all courses that are available. All this does is searches for all courses via Webreg's
    /// menu. Thus, only basic details are shown.
    ///
    /// # Parameters
    /// - `filter_by`: The request filter.
    ///
    /// # Returns
    /// A vector consisting of all courses that are available. Note that the data that is returned
    /// is directly from WebReg's API, so care will need to be taken to clean the resulting data.
    ///
    /// # Example
    /// Please see [`WebWegWrapper::search_courses_detailed`] for examples.
    pub async fn search_courses(
        &self,
        filter_by: SearchType<'_>,
    ) -> types::Result<Vec<RawWebRegSearchResultItem>> {
        process_get_result::<Vec<RawWebRegSearchResultItem>>(
            self.init_get_request(build_search_course_url(filter_by, self.info.term)?)
                .send()
                .await,
        )
        .await
    }

    /// Sends an email to yourself using the same email that is used to confirm that you have
    /// enrolled or waitlisted in a particular class. In other words, this will send an email
    /// to you through the email NoReplyRegistrar@ucsd.edu.
    ///
    /// It is strongly recommended that this function not be abused.
    ///
    /// # Parameters
    /// - `email_content`: The email to send.
    ///
    /// # Returns
    /// `true` if the email was sent successfully and `false` otherwise.
    ///
    /// # Example
    /// This will send an email to yourself with the content specified as the string shown below.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// let res = wrapper
    ///     .send_email_to_self("Hello, world! This will be sent to you via email.")
    ///     .await;
    ///
    /// match res {
    ///     Ok(_) => println!("Sent successfully."),
    ///     Err(e) => eprintln!("Error! {}", e)
    /// };
    /// # }
    /// ```
    pub async fn send_email_to_self(&self, email_content: &str) -> types::Result<()> {
        let r = self
            .info
            .client
            .post(SEND_EMAIL)
            .form(&[("actionevent", email_content), ("termcode", self.info.term)])
            .header(COOKIE, self.info.cookies)
            .header(USER_AGENT, self.info.user_agent)
            .send()
            .await?;

        if !r.status().is_success() {
            return Err(WrapperError::BadStatusCode(r.status().as_u16()));
        }

        let t = r.text().await?;
        if t.contains("\"YES\"") {
            Ok(())
        } else {
            Err(WrapperError::WebRegError(t))
        }
    }

    /// Changes the grading option for the class corresponding to the section ID.
    ///
    /// # Parameters
    /// - `section_id`: The section ID corresponding to the class that you want to change
    /// the grading option for.
    /// - `new_grade_opt`: The new grading option. This must either be `L` (letter),
    /// `P` (pass/no pass), or `S` (satisfactory/unsatisfactory), and is enforced via an enum.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Example
    /// Changing the section associated with section ID `12345` to letter grading option.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::types::GradeOption;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// let res = wrapper.change_grading_option("12345", GradeOption::L).await;
    ///
    /// match res {
    ///     Ok(_) => println!("Success!"),
    ///     Err(e) => eprintln!("Something went wrong: {}", e)
    /// }
    /// # }
    /// ```
    pub async fn change_grading_option(
        &self,
        section_id: &str,
        new_grade_opt: GradeOption,
    ) -> types::Result<bool> {
        let new_grade_opt = match new_grade_opt {
            GradeOption::L => "L",
            GradeOption::S => "S",
            GradeOption::P => "P",
        };

        // "Slice" any zeros off of the left-most side of the string. We need to do this
        // because, when comparing section IDs in the schedule, WebReg gives us the
        // section IDs as integers; however, for the rest of the API, it's given as a
        // string.
        //
        // Essentially, this means that, while most of WebReg's API will take `"079911"` as
        // an input and as an output (e.g. see `get_course_info`), the schedule API will
        // specifically return an integer `79911`. The `get_schedule` function will simply
        // convert this integer to a string, e.g. `79911` -> `"79911"` and return that along
        // with the other parsed info for each scheduled section.
        //
        // So, we need to slice off any 0s from the input parameter `section_id` to account
        // for this.
        let mut left_idx = 0;
        for c in section_id.chars() {
            if c != '0' {
                break;
            }

            left_idx += 1;
            continue;
        }

        let poss_class = self
            .get_schedule(None as Option<&str>)
            .await?
            .into_iter()
            .find(|x| x.section_id == section_id[left_idx..]);

        // don't care about previous poss_class
        let poss_class = match poss_class {
            Some(s) => s,
            None => return Err(WrapperError::GeneralError("Class not found.".into())),
        };

        let sec_id = poss_class.section_id.to_string();
        let units = poss_class.units.to_string();

        process_post_response(
            self.info
                .client
                .post(CHANGE_ENROLL)
                .form(&[
                    ("section", sec_id.as_str()),
                    ("subjCode", ""),
                    ("crseCode", ""),
                    ("unit", units.as_str()),
                    ("grade", new_grade_opt),
                    // You don't actually need these
                    ("oldGrade", ""),
                    ("oldUnit", ""),
                    ("termcode", self.info.term),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Validates that adding a course to your plan will cause no issue.
    ///
    /// # Parameters
    /// - `plan_options`: Information for the course that you want to plan.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// an issue appears.
    ///
    /// # Example
    /// Here, we will add the course `CSE 100`, which has section ID `079911` and section code
    /// `A01`, to our plan.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::types::{GradeOption, PlanAdd};
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    ///
    /// let res = wrapper.validate_add_to_plan(&PlanAdd {
    ///     subject_code: "CSE",
    ///     course_code: "100",
    ///     section_id: "079911",
    ///     section_code: "A01",
    ///     // Using S/U grading.
    ///     grading_option: Some(GradeOption::S),
    ///     // Put in default schedule
    ///     schedule_name: None,
    ///     unit_count: 4
    /// }).await;
    ///
    /// match res {
    ///     Ok(o) => println!("{}", if o { "Successful, planning is good" } else { "Unsuccessful" }),
    ///     Err(e) => eprintln!("{}", e),
    /// };
    /// # }
    /// ```
    pub async fn validate_add_to_plan(&self, plan_options: &PlanAdd<'_>) -> types::Result<bool> {
        let crsc_code = util::get_formatted_course_num(plan_options.course_code);
        process_post_response(
            self.info
                .client
                .post(PLAN_EDIT)
                .form(&[
                    ("section", plan_options.section_id),
                    ("subjcode", plan_options.subject_code),
                    ("crsecode", crsc_code.as_str()),
                    ("termcode", self.info.term),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Allows you to plan a course.
    ///
    /// # Parameters
    /// - `plan_options`: Information for the course that you want to plan.
    /// - `validate`: Whether to validate your planning of this course beforehand.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Warning
    /// Setting the `validate` parameter to `false` can cause issues. For example, when this is
    /// `false`, you will be able to plan courses with more units than allowed (e.g. 42 units), set
    /// the rading option to one that you are not allowed to use (e.g. S/U as an undergraduate),
    /// and only enroll in specific components of a section (e.g. just the discussion section).
    /// Some of these options can visually break WebReg (e.g. Remove/Enroll button will not appear).
    ///
    /// # Example
    /// Here, we will add the course `CSE 100`, which has section ID `079911` and section code
    /// `A01`, to our plan.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::{GradeOption, PlanAdd, WebRegWrapper};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    ///
    /// let res = wrapper.add_to_plan(PlanAdd {
    ///     subject_code: "CSE",
    ///     course_code: "100",
    ///     section_id: "079911",
    ///     section_code: "A01",
    ///     // Using S/U grading.
    ///     grading_option: Some(GradeOption::S),
    ///     // Put in default schedule
    ///     schedule_name: None,
    ///     unit_count: 4
    /// }, true).await;
    ///
    /// match res {
    ///     Ok(o) => println!("{}", if o { "Successful" } else { "Unsuccessful" }),
    ///     Err(e) => eprintln!("{}", e),
    /// };
    /// # }
    /// ```
    pub async fn add_to_plan(
        &self,
        plan_options: PlanAdd<'_>,
        validate: bool,
    ) -> types::Result<bool> {
        let u = plan_options.unit_count.to_string();
        let crsc_code = util::get_formatted_course_num(plan_options.course_code);

        if validate {
            // We need to call the edit endpoint first, or else we'll have issues where we don't
            // actually enroll in every component of the course.
            // Also, this can potentially return "false" due to you not being able to enroll in the
            // class, e.g. the class you're trying to plan is a major-restricted class.
            self.validate_add_to_plan(&plan_options)
                .await
                .unwrap_or(false);
        }

        process_post_response(
            self.info
                .client
                .post(PLAN_ADD)
                .form(&[
                    ("subjcode", plan_options.subject_code),
                    ("crsecode", crsc_code.as_str()),
                    ("sectnum", plan_options.section_id),
                    ("sectcode", plan_options.section_code),
                    ("unit", u.as_str()),
                    (
                        "grade",
                        match plan_options.grading_option {
                            Some(r) => match r {
                                GradeOption::L => "L",
                                GradeOption::S => "S",
                                GradeOption::P => "P",
                            },
                            _ => "L",
                        },
                    ),
                    ("termcode", self.info.term),
                    (
                        "schedname",
                        match plan_options.schedule_name {
                            Some(r) => r,
                            None => DEFAULT_SCHEDULE_NAME,
                        },
                    ),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Allows you to unplan a course.
    ///
    /// # Parameters
    /// - `section_id`: The section ID.
    /// - `schedule_name`: The schedule name where the course should be unplanned from.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Example
    /// Here, we will remove the planned course with section ID `079911` from our default schedule.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::{GradeOption, WebRegWrapper};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// let res = wrapper.remove_from_plan("079911", None).await;
    /// match res {
    ///     Ok(o) => println!("{}", if o { "Successful" } else { "Unsuccessful" }),
    ///     Err(e) => eprintln!("{}", e),
    /// };
    /// # }
    /// ```
    pub async fn remove_from_plan(
        &self,
        section_id: impl AsRef<str>,
        schedule_name: Option<&str>,
    ) -> types::Result<bool> {
        process_post_response(
            self.info
                .client
                .post(PLAN_REMOVE)
                .form(&[
                    ("sectnum", section_id.as_ref()),
                    ("termcode", self.info.term),
                    ("schedname", schedule_name.unwrap_or(DEFAULT_SCHEDULE_NAME)),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Validates that the section that you are trying to enroll in is valid.
    ///
    /// # Parameters
    /// - `is_enroll`: Whether you are enrolling.
    /// - `enroll_options`: The enrollment options. Note that the section ID is the only thing
    /// that matters here. A reference, thus, is expected since you will probably be reusing
    /// the structure when calling the `add_section` function.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// there is an issue when trying to enroll.
    ///
    /// # Example
    /// Here, we will enroll in the course with section ID `078616`, and with the default grading
    /// option and unit count.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::types::EnrollWaitAdd;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    ///
    /// let enroll_options = EnrollWaitAdd {
    ///     section_id: "078616",
    ///     // Use default grade option
    ///     grading_option: None,
    ///     // Use default unit count
    ///     unit_count: None,
    /// };
    ///
    /// let add_res = wrapper
    ///     .validate_add_section(
    ///         // Use true here since we want to enroll (not waitlist). Note that this might
    ///         // result in an `Err` being returned if you can't enroll.
    ///         true,
    ///         &enroll_options,
    ///     )
    ///     .await;
    ///
    /// match add_res {
    ///     Ok(o) => println!("{}", if o { "Successful" } else { "Unsuccessful" }),
    ///     Err(e) => eprintln!("{}", e),
    /// };
    /// # }
    /// ```
    pub async fn validate_add_section(
        &self,
        is_enroll: bool,
        enroll_options: &EnrollWaitAdd<'_>,
    ) -> types::Result<bool> {
        let base_edit_url = if is_enroll {
            ENROLL_EDIT
        } else {
            WAITLIST_EDIT
        };

        process_post_response(
            self.info
                .client
                .post(base_edit_url)
                .form(&[
                    // These are required
                    ("section", enroll_options.section_id),
                    ("termcode", self.info.term),
                    // These are optional.
                    ("subjcode", ""),
                    ("crsecode", ""),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Enrolls in, or waitlists, a class.
    ///
    /// # Parameters
    /// - `is_enroll`: Whether you want to enroll. This should be `true` if you want to enroll
    /// in this section and `false` if you want to waitlist.
    /// - `enroll_options`: Information for the course that you want to enroll in.
    /// - `validate`: Whether to validate your enrollment of this course beforehand. Note that
    /// validation is required, so this should be `true`. This should only be `false` if you
    /// called `validate_add_section` before. If you attempt to call `add_section` without
    /// validation, then you will get an error.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Example
    /// Here, we will enroll in the course with section ID `078616`, and with the default grading
    /// option and unit count.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::{EnrollWaitAdd, WebRegWrapper};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    ///
    /// let add_res = wrapper
    ///     .add_section(
    ///         // Use true here since we want to enroll (not waitlist). Note that this might
    ///         // result in an `Err` being returned if you can't enroll.
    ///         true,
    ///         EnrollWaitAdd {
    ///             section_id: "078616",
    ///             // Use default grade option
    ///             grading_option: None,
    ///             // Use default unit count
    ///             unit_count: None,
    ///         },
    ///         true,
    ///     )
    ///     .await;
    ///
    /// match add_res {
    ///     Ok(o) => println!("{}", if o { "Successful" } else { "Unsuccessful" }),
    ///     Err(e) => eprintln!("{}", e),
    /// };
    /// # }
    /// ```
    pub async fn add_section(
        &self,
        is_enroll: bool,
        enroll_options: EnrollWaitAdd<'_>,
        validate: bool,
    ) -> types::Result<bool> {
        let base_reg_url = if is_enroll { ENROLL_ADD } else { WAITLIST_ADD };
        let u = match enroll_options.unit_count {
            Some(r) => r.to_string(),
            None => "".to_string(),
        };

        if validate {
            self.validate_add_section(is_enroll, &enroll_options)
                .await?;
        }

        process_post_response(
            self.info
                .client
                .post(base_reg_url)
                .form(&[
                    // These are required
                    ("section", enroll_options.section_id),
                    ("termcode", self.info.term),
                    // These are optional.
                    ("unit", u.as_str()),
                    (
                        "grade",
                        match enroll_options.grading_option {
                            Some(r) => match r {
                                GradeOption::L => "L",
                                GradeOption::S => "S",
                                GradeOption::P => "P",
                            },
                            _ => "",
                        },
                    ),
                    ("crsecode", ""),
                    ("subjcode", ""),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await?;

        // This will always return true
        process_post_response(
            self.info
                .client
                .post(PLAN_REMOVE_ALL)
                .form(&[
                    ("sectnum", enroll_options.section_id),
                    ("termcode", self.info.term),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Drops a section.
    ///
    /// # Parameters
    /// - `was_enrolled`: Whether you were originally enrolled in the section. This would
    /// be `true` if you were enrolled and `false` if waitlisted.
    /// - `section_id`: The section ID corresponding to the section that you want to drop.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Remarks
    /// It is a good idea to make a call to get your current schedule before you
    /// make a request here. That way, you know which classes can be dropped.
    ///
    /// # Example
    /// Here, we assume that we are enrolled in a course with section ID `078616`, and want to
    /// drop it.
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    ///
    /// // Use `true` here since we were originally enrolled (not waitlisted).
    /// let drop_res = wrapper.drop_section(true, "078616").await;
    /// match drop_res {
    ///     Ok(o) => println!("{}", if o { "Successful" } else { "Unsuccessful" }),
    ///     Err(e) => eprintln!("{}", e),
    /// };
    /// # }
    /// ```
    pub async fn drop_section(
        &self,
        was_enrolled: bool,
        section_id: impl AsRef<str>,
    ) -> types::Result<bool> {
        let base_reg_url = if was_enrolled {
            ENROLL_DROP
        } else {
            WAILIST_DROP
        };

        process_post_response(
            self.info
                .client
                .post(base_reg_url)
                .form(&[
                    // These parameters are optional
                    ("subjcode", ""),
                    ("crsecode", ""),
                    // But these are required
                    ("section", section_id.as_ref()),
                    ("termcode", self.info.term),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Renames a schedule to the specified name. You cannot rename the default
    /// `My Schedule` schedule.
    ///
    /// # Parameter
    /// - `old_name`: The name of the old schedule.
    /// - `new_name`: The name that you want to change the old name to.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Example
    /// Renaming the schedule "`Test Schedule`" to "`Another Schedule`."
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// // You should do error handling here, but I won't
    /// assert!(!wrapper.get_schedule_list().await.unwrap().contains(&"Another Schedule".to_string()));
    /// wrapper.rename_schedule("Test Schedule", "Another Schedule").await.expect("an error occurred");
    /// assert!(wrapper.get_schedule_list().await.unwrap().contains(&"Another Schedule".to_string()));
    /// # }
    /// ```
    pub async fn rename_schedule(
        &self,
        old_name: impl AsRef<str>,
        new_name: impl AsRef<str>,
    ) -> types::Result<bool> {
        // Can't rename your default schedule.
        if old_name.as_ref() == DEFAULT_SCHEDULE_NAME {
            return Err(WrapperError::InputError(
                "old_name",
                "You cannot rename the default schedule",
            ));
        }

        process_post_response(
            self.info
                .client
                .post(RENAME_SCHEDULE)
                .form(&[
                    ("termcode", self.info.term),
                    ("oldschedname", old_name.as_ref()),
                    ("newschedname", new_name.as_ref()),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Removes a schedule. You cannot delete the default `My Schedule` one.
    ///
    /// # Parameter
    /// - `schedule_name`: The name of the schedule to delete.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Example
    /// Delete the schedule "`Test Schedule`."
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// // You should do error handling here, but I won't
    /// assert!(wrapper.get_schedule_list().await.unwrap().contains(&"Test Schedule".to_string()));
    /// wrapper.remove_schedule("Test Schedule").await.expect("an error occurred");
    /// assert!(!wrapper.get_schedule_list().await.unwrap().contains(&"Test Schedule".to_string()));
    /// # }
    /// ```
    pub async fn remove_schedule(&self, schedule_name: impl AsRef<str>) -> types::Result<bool> {
        // Can't remove your default schedule.
        if schedule_name.as_ref() == DEFAULT_SCHEDULE_NAME {
            return Err(WrapperError::InputError(
                "schedule_name",
                "You cannot remove the default schedule.",
            ));
        }

        process_post_response(
            self.info
                .client
                .post(REMOVE_SCHEDULE)
                .form(&[
                    ("termcode", self.info.term),
                    ("schedname", schedule_name.as_ref()),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Adds an event to your WebReg calendar, or edits an existing event.
    ///
    /// # Parameter
    /// - `event_info`: The details of the event.
    /// - `event_timestamp`: The timestamp corresponding to the event that you want to
    /// edit. If this is `None`, then this function will add the event. If this is `Some`,
    /// then this function will edit an existing event.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Example
    /// Renaming the schedule "`Test Schedule`" to "`Another Schedule`."
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::types::{DayOfWeek, EventAdd};
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// let event = EventAdd {
    ///     event_name: "Clown on AYU",
    ///     location: Some("B250"),
    ///     event_days: vec![DayOfWeek::Monday, DayOfWeek::Friday],
    ///     start_hr: 5,
    ///     start_min: 30,
    ///     end_hr: 10,
    ///     end_min: 45,
    /// };
    ///
    /// // Adding an event
    /// wrapper.add_or_edit_event(event, None).await.expect("an error occurred");
    ///
    /// // Editing an event (commenting this out since we moved `event` in the previous line)
    /// // wrapper.add_or_edit_event(event, Some("2022-09-09 21:50:16.846885")).await;
    /// # }
    /// ```
    pub async fn add_or_edit_event(
        &self,
        event_info: EventAdd<'_>,
        event_timestamp: impl Into<Option<&str>>,
    ) -> types::Result<bool> {
        let start_time_full = event_info.start_hr * 100 + event_info.start_min;
        let end_time_full = event_info.end_hr * 100 + event_info.end_min;
        if start_time_full >= end_time_full {
            return Err(WrapperError::InputError(
                "time",
                "Start time must be less than end time.",
            ));
        }

        if event_info.start_hr < 7 || event_info.start_hr > 12 + 10 {
            return Err(WrapperError::InputError(
                "event_info.start_hr",
                "Start hour must be between 7 and 22 (7am and 10pm)",
            ));
        }

        if event_info.start_hr == 12 + 10 && event_info.start_min != 0 {
            return Err(WrapperError::InputError(
                "event_info.start",
                "You cannot exceed 10pm.",
            ));
        }

        if event_info.event_days.is_empty() {
            return Err(WrapperError::InputError(
                "event_info.event_days",
                "Must specify one day.",
            ));
        }

        let mut days: [bool; 7] = [false; 7];
        for d in event_info.event_days {
            let idx = match d {
                DayOfWeek::Monday => 0,
                DayOfWeek::Tuesday => 1,
                DayOfWeek::Wednesday => 2,
                DayOfWeek::Thursday => 3,
                DayOfWeek::Friday => 4,
                DayOfWeek::Saturday => 5,
                DayOfWeek::Sunday => 6,
            };

            days[idx] = true;
        }

        let mut day_str = String::new();
        for d in days {
            day_str.push(if d { '1' } else { '0' });
        }

        assert_eq!(7, day_str.len());

        let mut start_time_full = start_time_full.to_string();
        let mut end_time_full = end_time_full.to_string();
        while start_time_full.len() < 4 {
            start_time_full.insert(0, '0');
        }

        while end_time_full.len() < 4 {
            end_time_full.insert(0, '0');
        }

        let mut form_data = HashMap::from([
            ("termcode", self.info.term),
            ("aename", event_info.event_name),
            ("aestarttime", start_time_full.as_str()),
            ("aeendtime", end_time_full.as_str()),
            ("aelocation", event_info.location.unwrap_or("")),
            ("aedays", day_str.as_str()),
        ]);

        let et = event_timestamp.into();
        if let Some(timestamp) = et {
            form_data.insert("aetimestamp", timestamp);
        }

        process_post_response(
            self.info
                .client
                .post(match et {
                    Some(_) => EVENT_EDIT,
                    None => EVENT_ADD,
                })
                .form(&form_data)
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Removes an event from your WebReg calendar.
    ///
    /// # Parameter
    /// - `event_timestamp`: The timestamp corresponding to the event that you want to
    /// remove.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Example
    /// Renaming the schedule "`Test Schedule`" to "`Another Schedule`."
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// // Removing an event
    /// wrapper.remove_event("2022-09-09 21:50:16.846885").await.expect("an error occurred");
    /// # }
    /// ```
    pub async fn remove_event(&self, event_timestamp: impl AsRef<str>) -> types::Result<bool> {
        process_post_response(
            self.info
                .client
                .post(EVENT_REMOVE)
                .form(&[
                    ("aetimestamp", event_timestamp.as_ref()),
                    ("termcode", self.info.term),
                ])
                .header(COOKIE, self.info.cookies)
                .header(USER_AGENT, self.info.user_agent)
                .send()
                .await,
        )
        .await
    }

    /// Gets all event from your WebReg calendar.
    ///
    /// # Returns
    /// A vector of all events, or `None` if an error occurred.
    ///
    /// # Example
    /// Renaming the schedule "`Test Schedule`" to "`Another Schedule`."
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webweg::wrapper::WebRegWrapper;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let wrapper = WebRegWrapper::new(Client::new(), "my cookies".to_string(), "FA22");
    /// // Get all my events
    /// let all_events = wrapper.get_events().await;
    /// # }
    /// ```
    pub async fn get_events(&self) -> types::Result<Vec<Event>> {
        let url = Url::parse_with_params(EVENT_GET, &[("termcode", self.info.term)]).unwrap();
        parse_get_events(
            process_get_result::<Vec<RawEvent>>(self.init_get_request(url).send().await).await?,
        )
    }

    /// Gets all of your schedules.
    ///
    /// # Returns
    /// Either a vector of strings representing the names of the schedules, or the error that
    /// occurred.
    pub async fn get_schedule_list(&self) -> types::Result<Vec<String>> {
        let url = Url::parse_with_params(ALL_SCHEDULE, &[("termcode", self.info.term)])?;

        process_get_result::<Vec<String>>(self.init_get_request(url).send().await).await
    }

    /// Initializes a GET `RequestBuilder` with the cookies and user agent specified.
    ///
    /// # Parameters
    /// - `url`: The URL to make the request for.
    ///
    /// # Returns
    /// The GET `RequestBuilder`.
    fn init_get_request(&self, url: Url) -> RequestBuilder {
        self.info
            .client
            .get(url)
            .header(COOKIE, self.info.cookies)
            .header(USER_AGENT, self.info.user_agent)
    }
}
