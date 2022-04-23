use crate::webreg_clean_defn::{
    CourseSection, EnrollmentStatus, Meeting, MeetingDay, ScheduledSection,
};
use crate::webreg_helper;
use crate::webreg_raw_defn::{RawScheduledMeeting, RawWebRegMeeting, RawWebRegSearchResultItem};
use reqwest::header::{COOKIE, USER_AGENT};
use reqwest::{Client, Error, Response};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::cmp::max;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::SystemTime;
use url::Url;

// URLs for WebReg
const MY_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, \
like Gecko) Chrome/97.0.4692.71 Safari/537.36";

const DEFAULT_SCHEDULE_NAME: &str = "My Schedule";

// Random WebReg links
const WEBREG_BASE: &str = "https://act.ucsd.edu/webreg2";
const WEBREG_SEARCH: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/search-by-all?";
const WEBREG_SEARCH_SEC: &str =
    "https://act.ucsd.edu/webreg2/svc/wradapter/secure/search-by-sectionid?";
const ACC_NAME: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/get-current-name";
const COURSE_DATA: &str =
    "https://act.ucsd.edu/webreg2/svc/wradapter/secure/search-load-group-data?";
const CURR_SCHEDULE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/get-class?";
const SEND_EMAIL: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/send-email";
const CHANGE_ENROLL: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/change-enroll";

const REMOVE_SCHEDULE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/sched-remove";
const RENAME_SCHEDULE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/plan-rename";
const ALL_SCHEDULE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/sched-get-schednames";

const PING_SERVER: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/ping-server";

const PLAN_ADD: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/plan-add";
const PLAN_REMOVE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/plan-remove";
const PLAN_EDIT: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/edit-plan";
const PLAN_REMOVE_ALL: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/plan-remove-all";

const ENROLL_ADD: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/add-enroll";
const ENROLL_EDIT: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/edit-enroll";
const ENROLL_DROP: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/drop-enroll";

const WAITLIST_ADD: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/add-wait";
const WAITLIST_EDIT: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/edit-wait";
const WAILIST_DROP: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/drop-wait";

/// The generic type is the return value. Otherwise, regardless of request type,
/// we're just returning the error string if there is an error.
pub type Output<'a, T> = Result<T, Cow<'a, str>>;

/// A wrapper for [UCSD's WebReg](https://act.ucsd.edu/webreg2/start). For more information,
/// please see the README.
pub struct WebRegWrapper<'a> {
    cookies: String,
    client: Client,
    term: &'a str,
}

impl<'a> WebRegWrapper<'a> {
    /// Creates a new instance of the `WebRegWrapper`.
    ///
    /// # Parameters
    /// - `cookies`: The cookies from your session of WebReg.
    /// - `term`: The term.
    ///
    /// # Returns
    /// The new instance.
    pub fn new(cookies: String, term: &'a str) -> Self {
        WebRegWrapper {
            cookies,
            client: Client::new(),
            term,
        }
    }

    /// Creates a new instance of the `WebRegWrapper` with a custom client.
    ///
    /// # Parameters
    /// - `cookies`: The cookies from your session of WebReg.
    /// - `term`: The term.
    /// - `client`: The client.
    ///
    /// # Returns
    /// The new instance.
    pub fn new_with_client(cookies: String, term: &'a str, client: Client) -> Self {
        WebRegWrapper {
            cookies,
            client,
            term,
        }
    }

    /// Sets the cookies to the new, specified cookies.
    ///
    /// # Parameters
    /// - `new_cookies`: The new cookies.
    pub fn set_cookies(&mut self, new_cookies: String) {
        self.cookies = new_cookies;
    }

    /// Checks if the current WebReg instance is valid.
    ///
    /// # Returns
    /// `true` if the instance is valid and `false` otherwise.
    pub async fn is_valid(&self) -> bool {
        let res = self
            .client
            .get(WEBREG_BASE)
            .header(COOKIE, &self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => false,
            Ok(r) => self._internal_is_valid(&r.text().await.unwrap()),
        }
    }

    /// Gets the name of the owner associated with this account.
    ///
    /// # Returns
    /// The name of the person, or an empty string if the cookies that were given were invalid.
    pub async fn get_account_name(&self) -> Cow<'a, str> {
        let res = self
            .client
            .get(ACC_NAME)
            .header(COOKIE, &self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => "".into(),
            Ok(r) => {
                let name = r.text().await.unwrap();
                if self._internal_is_valid(&name) {
                    name.into()
                } else {
                    "".into()
                }
            }
        }
    }

    /// Gets your current schedule.
    ///
    /// # Parameters
    /// - `schedule_name`: The schedule that you want to get. If `None` is given, this will default
    /// to your main schedule.
    ///
    /// # Returns
    /// A result that can either be one of:
    /// - A vector of sections that appear in your schedule.
    /// - Or, the error message.
    pub async fn get_schedule(
        &self,
        schedule_name: Option<&str>,
    ) -> Output<'a, Vec<ScheduledSection>> {
        let url = Url::parse_with_params(
            CURR_SCHEDULE,
            &[
                ("schedname", schedule_name.unwrap_or(DEFAULT_SCHEDULE_NAME)),
                ("final", ""),
                ("sectnum", ""),
                ("termcode", self.term),
                ("_", self._get_epoch_time().to_string().as_str()),
            ],
        )
        .unwrap();

        let res = self
            ._process_get_result::<Vec<RawScheduledMeeting>>(
                self.client
                    .get(url)
                    .header(COOKIE, &self.cookies)
                    .header(USER_AGENT, MY_USER_AGENT)
                    .send()
                    .await,
            )
            .await?;

        if res.is_empty() {
            return Ok(vec![]);
        }

        let mut base_group_secs: HashMap<&str, Vec<&RawScheduledMeeting>> = HashMap::new();
        let mut special_classes: HashMap<&str, Vec<&RawScheduledMeeting>> = HashMap::new();
        for s_meeting in &res {
            if s_meeting.enrolled_count == Some(0) && s_meeting.section_capacity == Some(0) {
                continue;
            }

            if s_meeting.sect_code.as_bytes()[0].is_ascii_digit() {
                special_classes
                    .entry(s_meeting.course_title.trim())
                    .or_insert_with(Vec::new)
                    .push(s_meeting);

                continue;
            }

            base_group_secs
                .entry(s_meeting.course_title.trim())
                .or_insert_with(Vec::new)
                .push(s_meeting);
        }

        let mut schedule: Vec<ScheduledSection> = vec![];

        for (_, sch_meetings) in base_group_secs {
            let instructors = self._get_all_instructors(
                sch_meetings
                    .iter()
                    .flat_map(|x| self._get_instructor_names(&x.person_full_name)),
            );

            // Literally all just to find the "main" lecture since webreg is inconsistent
            // plus some courses may not have a lecture.
            let all_main = sch_meetings
                .iter()
                .filter(|x| {
                    x.sect_code.ends_with("00")
                        && x.special_meeting.replace("TBA", "").trim().is_empty()
                })
                .collect::<Vec<_>>();
            assert!(
                !all_main.is_empty()
                    && all_main
                        .iter()
                        .all(|x| x.meeting_type == all_main[0].meeting_type)
            );

            let mut all_meetings: Vec<Meeting> = vec![];
            for main in all_main {
                all_meetings.push(Meeting {
                    meeting_type: main.meeting_type.to_string(),
                    meeting_days: if main.day_code.trim().is_empty() {
                        MeetingDay::None
                    } else {
                        MeetingDay::Repeated(webreg_helper::parse_day_code(main.day_code.trim()))
                    },
                    start_min: main.start_time_min,
                    start_hr: main.start_time_hr,
                    end_min: main.end_time_min,
                    end_hr: main.end_time_hr,
                    building: main.bldg_code.trim().to_string(),
                    room: main.room_code.trim().to_string(),
                    other_instructors: vec![],
                });
            }

            // Calculate the remaining meetings. other_special consists of midterms and
            // final exams, for example, since they are all shared in the same overall
            // section (e.g. A02 & A03 are in A00)
            sch_meetings
                .iter()
                .filter(|x| {
                    x.sect_code.ends_with("00")
                        && !x.special_meeting.replace("TBA", "").trim().is_empty()
                })
                .map(|x| Meeting {
                    meeting_type: x.meeting_type.to_string(),
                    meeting_days: MeetingDay::OneTime(x.start_date.to_string()),
                    start_min: x.start_time_min,
                    start_hr: x.start_time_hr,
                    end_min: x.end_time_min,
                    end_hr: x.end_time_hr,
                    building: x.bldg_code.trim().to_string(),
                    room: x.room_code.trim().to_string(),
                    other_instructors: vec![],
                })
                .for_each(|meeting| all_meetings.push(meeting));

            // Other meetings
            sch_meetings
                .iter()
                .filter(|x| !x.sect_code.ends_with("00"))
                .map(|x| Meeting {
                    meeting_type: x.meeting_type.to_string(),
                    meeting_days: MeetingDay::Repeated(webreg_helper::parse_day_code(&x.day_code)),
                    start_min: x.start_time_min,
                    start_hr: x.start_time_hr,
                    end_min: x.end_time_min,
                    end_hr: x.end_time_hr,
                    building: x.bldg_code.trim().to_string(),
                    room: x.room_code.trim().to_string(),
                    other_instructors: vec![],
                })
                .for_each(|meeting| all_meetings.push(meeting));

            // Look for current waitlist count
            let wl_count = match sch_meetings.iter().find(|x| x.count_on_waitlist.is_some()) {
                Some(r) => r.count_on_waitlist.unwrap(),
                None => 0,
            };

            let pos_on_wl = if sch_meetings[0].enroll_status == "WT" {
                match sch_meetings
                    .iter()
                    .find(|x| x.waitlist_pos.chars().all(|y| y.is_numeric()))
                {
                    Some(r) => r.waitlist_pos.parse::<i64>().unwrap(),
                    None => 0,
                }
            } else {
                0
            };

            let enrolled_count = match sch_meetings.iter().find(|x| x.enrolled_count.is_some()) {
                Some(r) => r.enrolled_count.unwrap(),
                None => -1,
            };

            let section_capacity = match sch_meetings.iter().find(|x| x.section_capacity.is_some())
            {
                Some(r) => r.section_capacity.unwrap(),
                None => -1,
            };

            schedule.push(ScheduledSection {
                section_number: sch_meetings[0].section_number.to_string(),
                instructor: instructors.clone(),
                subject_code: sch_meetings[0].subj_code.trim().to_string(),
                course_code: sch_meetings[0].course_code.trim().to_string(),
                course_title: sch_meetings[0].course_title.trim().to_string(),
                section_code: match sch_meetings.iter().find(|x| !x.sect_code.ends_with("00")) {
                    Some(r) => r.sect_code.to_string(),
                    None => sch_meetings[0].sect_code.to_string(),
                },
                section_capacity,
                enrolled_count,
                available_seats: max(section_capacity - enrolled_count, 0),
                grade_option: sch_meetings[0].grade_option.trim().to_string(),
                units: sch_meetings[0].sect_credit_hrs,
                enrolled_status: match &*sch_meetings[0].enroll_status {
                    "EN" => EnrollmentStatus::Enrolled,
                    "WT" => EnrollmentStatus::Waitlist(pos_on_wl),
                    "PL" => EnrollmentStatus::Planned,
                    _ => EnrollmentStatus::Planned,
                },
                waitlist_ct: wl_count,
                meetings: all_meetings,
            });
        }

        // Classes with only a lecture
        for (_, sch_meetings) in special_classes {
            let day_code = sch_meetings
                .iter()
                .map(|x| x.day_code.trim())
                .collect::<Vec<_>>()
                .join("");

            let parsed_day_code = if day_code.is_empty() {
                MeetingDay::None
            } else {
                MeetingDay::Repeated(webreg_helper::parse_day_code(&day_code))
            };

            let section_capacity = sch_meetings[0].section_capacity.unwrap_or(-1);
            let enrolled_count = sch_meetings[0].enrolled_count.unwrap_or(-1);

            schedule.push(ScheduledSection {
                section_number: sch_meetings[0].section_number.to_string(),
                instructor: self._get_all_instructors(
                    sch_meetings
                        .iter()
                        .flat_map(|x| self._get_instructor_names(&x.person_full_name)),
                ),
                subject_code: sch_meetings[0].subj_code.trim().to_string(),
                course_code: sch_meetings[0].course_code.trim().to_string(),
                course_title: sch_meetings[0].course_title.trim().to_string(),
                section_code: sch_meetings[0].sect_code.to_string(),
                section_capacity,
                enrolled_count,
                available_seats: max(section_capacity - enrolled_count, 0),
                grade_option: sch_meetings[0].grade_option.trim().to_string(),
                units: sch_meetings[0].sect_credit_hrs,
                enrolled_status: match &*sch_meetings[0].enroll_status {
                    "EN" => EnrollmentStatus::Enrolled,
                    "WT" => EnrollmentStatus::Waitlist(-1),
                    "PL" => EnrollmentStatus::Planned,
                    _ => EnrollmentStatus::Planned,
                },
                waitlist_ct: -1,
                meetings: vec![Meeting {
                    meeting_type: sch_meetings[0].meeting_type.to_string(),
                    meeting_days: parsed_day_code,
                    start_min: sch_meetings[0].start_time_min,
                    start_hr: sch_meetings[0].start_time_hr,
                    end_min: sch_meetings[0].end_time_min,
                    end_hr: sch_meetings[0].start_time_hr,
                    building: sch_meetings[0].bldg_code.trim().to_string(),
                    room: sch_meetings[0].room_code.trim().to_string(),
                    other_instructors: vec![],
                }],
            });
        }

        Ok(schedule)
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
    /// - `course_code`: The course code. For example, if you wanted to check `MATH 100B`, you
    /// would put `100B`.
    ///
    /// # Returns
    /// A result containing either:
    /// - A vector with all possible sections that match the given subject code & course code.
    /// - Or the error that occurred.
    pub async fn get_enrollment_count(
        &self,
        subject_code: &str,
        course_code: &str,
    ) -> Output<'a, Vec<CourseSection>> {
        let crsc_code = self._get_formatted_course_code(course_code);
        let url = Url::parse_with_params(
            COURSE_DATA,
            &[
                ("subjcode", subject_code),
                ("crsecode", &*crsc_code),
                ("termcode", self.term),
                ("_", self._get_epoch_time().to_string().as_str()),
            ],
        )
        .unwrap();

        let meetings = self
            ._process_get_result::<Vec<RawWebRegMeeting>>(
                self.client
                    .get(url)
                    .header(COOKIE, &self.cookies)
                    .header(USER_AGENT, MY_USER_AGENT)
                    .send()
                    .await,
            )
            .await?;

        let mut meetings_to_parse = vec![];
        let mut seen: HashSet<&str> = HashSet::new();
        for meeting in &meetings {
            if !seen.insert(&*meeting.sect_code) {
                continue;
            }

            meetings_to_parse.push(meeting);
        }

        Ok(meetings_to_parse
            .into_iter()
            .filter(|x| x.display_type == "AC")
            .map(|x| CourseSection {
                subj_course_id: format!("{} {}", subject_code.trim(), course_code.trim())
                    .to_uppercase(),
                section_id: x.section_number.trim().to_string(),
                section_code: x.sect_code.trim().to_string(),
                instructors: self._get_instructor_names(&x.person_full_name),
                available_seats: max(x.avail_seat, 0),
                enrolled_ct: x.enrolled_count,
                total_seats: x.section_capacity,
                waitlist_ct: x.count_on_waitlist,
                meetings: vec![],
                needs_waitlist: x.needs_waitlist == "Y",
            })
            .collect())
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
    /// - `course_code`: The course code. For example, if you wanted to check `MATH 100B`, you
    /// would put `100B`.
    ///
    /// # Returns
    /// A result containing either:
    /// - A vector with all possible sections that match the given subject code & course code.
    /// - Or the error that occurred.
    pub async fn get_course_info(
        &self,
        subject_code: &str,
        course_code: &str,
    ) -> Output<'a, Vec<CourseSection>> {
        let crsc_code = self._get_formatted_course_code(course_code);
        let url = Url::parse_with_params(
            COURSE_DATA,
            &[
                ("subjcode", subject_code),
                ("crsecode", &*crsc_code),
                ("termcode", self.term),
                ("_", self._get_epoch_time().to_string().as_str()),
            ],
        )
        .unwrap();

        let parsed = self
            ._process_get_result::<Vec<RawWebRegMeeting>>(
                self.client
                    .get(url)
                    .header(COOKIE, &self.cookies)
                    .header(USER_AGENT, MY_USER_AGENT)
                    .send()
                    .await,
            )
            .await?;

        let course_dept_id =
            format!("{} {}", subject_code.trim(), course_code.trim()).to_uppercase();

        // Process any "special" sections. Special sections are sections whose section code is just
        // numbers, e.g. section 001.
        let mut sections: Vec<CourseSection> = vec![];
        let mut unprocessed_sections: Vec<RawWebRegMeeting> = vec![];
        for webreg_meeting in parsed {
            if !webreg_helper::is_valid_meeting(&webreg_meeting) {
                continue;
            }

            // If section code starts with a number then it's probably a special section.
            if webreg_meeting.sect_code.as_bytes()[0].is_ascii_digit() {
                let m = webreg_helper::parse_meeting_type_date(&webreg_meeting);

                sections.push(CourseSection {
                    subj_course_id: course_dept_id.clone(),
                    section_id: webreg_meeting.section_number.trim().to_string(),
                    section_code: webreg_meeting.sect_code.trim().to_string(),
                    instructors: vec![webreg_meeting
                        .person_full_name
                        .split_once(';')
                        .unwrap()
                        .0
                        .trim()
                        .to_string()],
                    // Because it turns out that you can have negative available seats.
                    available_seats: max(webreg_meeting.avail_seat, 0),
                    enrolled_ct: webreg_meeting.enrolled_count,
                    total_seats: webreg_meeting.section_capacity,
                    waitlist_ct: webreg_meeting.count_on_waitlist,
                    needs_waitlist: webreg_meeting.needs_waitlist == "Y",
                    meetings: vec![Meeting {
                        start_hr: webreg_meeting.start_time_hr,
                        start_min: webreg_meeting.start_time_min,
                        end_hr: webreg_meeting.end_time_hr,
                        end_min: webreg_meeting.end_time_min,
                        meeting_type: m.0.to_string(),
                        meeting_days: m.1,
                        building: webreg_meeting.bldg_code.trim().to_string(),
                        room: webreg_meeting.room_code.trim().to_string(),
                        other_instructors: vec![],
                    }],
                });

                continue;
            }

            // If the component cannot be enrolled in,
            // AND the section code doesn't end with '00'
            // Then it's useless for us
            if webreg_meeting.display_type != "AC" && !webreg_meeting.sect_code.ends_with("00") {
                continue;
            }

            unprocessed_sections.push(webreg_meeting);
        }

        if unprocessed_sections.is_empty() {
            return Ok(sections);
        }

        // Process remaining sections
        let mut all_groups: Vec<GroupedSection<RawWebRegMeeting>> = vec![];
        let mut sec_main_ids = unprocessed_sections
            .iter()
            .filter(|x| x.sect_code.ends_with("00"))
            .map(|x| &*x.sect_code)
            .collect::<VecDeque<_>>();

        let mut seen: HashSet<&str> = HashSet::new();
        while !sec_main_ids.is_empty() {
            let main_id = sec_main_ids.pop_front().unwrap();
            if seen.contains(main_id) {
                continue;
            }

            seen.insert(main_id);
            let letter = main_id.chars().into_iter().next().unwrap();
            let mut group = GroupedSection {
                main_meeting: vec![],
                child_meetings: vec![],
                other_special_meetings: vec![],
            };

            unprocessed_sections
                .iter()
                .filter(|x| {
                    x.sect_code == main_id && x.special_meeting.replace("TBA", "").trim().is_empty()
                })
                .for_each(|x| group.main_meeting.push(x));

            if group.main_meeting.is_empty() {
                continue;
            }

            // Want all sections with section code starting with the same letter as what
            // the main section code is. So, if main_id is A00, we want all sections that
            // have section code starting with A.
            unprocessed_sections
                .iter()
                .filter(|x| x.sect_code.starts_with(letter))
                .for_each(|x| {
                    // Don't count this again
                    let special_meeting = x.special_meeting.replace("TBA", "");
                    if x.sect_code == main_id && special_meeting.trim().is_empty() {
                        return;
                    }

                    // Probably a discussion
                    // Original if-condition:
                    // (x.start_date == x.section_start_date && special_meeting.trim().is_empty())
                    if x.sect_code != main_id {
                        group.child_meetings.push(x);
                        return;
                    }

                    group.other_special_meetings.push(x);
                });

            all_groups.push(group);
        }

        // Process each group
        for group in all_groups {
            let base_instructors = self._get_all_instructors(
                vec![
                    group
                        .main_meeting
                        .iter()
                        .flat_map(|x| self._get_instructor_names(&x.person_full_name))
                        .collect::<Vec<_>>(),
                    group
                        .other_special_meetings
                        .iter()
                        .flat_map(|x| self._get_instructor_names(&x.person_full_name))
                        .collect::<Vec<_>>(),
                ]
                .concat()
                .into_iter(),
            );

            let mut main_meetings: Vec<Meeting> = vec![];
            for meeting in &group.main_meeting {
                let (m_m_type, m_days) = webreg_helper::parse_meeting_type_date(meeting);

                main_meetings.push(Meeting {
                    meeting_type: m_m_type.to_string(),
                    meeting_days: m_days,
                    building: meeting.bldg_code.trim().to_string(),
                    room: meeting.room_code.trim().to_string(),
                    start_hr: meeting.start_time_hr,
                    start_min: meeting.start_time_min,
                    end_hr: meeting.end_time_hr,
                    end_min: meeting.end_time_min,
                    // Main meetings should only ever have the base instructors. In other words,
                    // the professor assigned to teach section X00 should be the only one here.
                    other_instructors: vec![],
                });
            }

            let other_meetings = group
                .other_special_meetings
                .into_iter()
                .map(|x| {
                    let (o_m_type, o_days) = webreg_helper::parse_meeting_type_date(x);
                    Meeting {
                        meeting_type: o_m_type.to_string(),
                        meeting_days: o_days,
                        building: x.bldg_code.trim().to_string(),
                        room: x.room_code.trim().to_string(),
                        start_hr: x.start_time_hr,
                        start_min: x.start_time_min,
                        end_hr: x.end_time_hr,
                        end_min: x.end_time_min,
                        // Same idea as with the justification above
                        other_instructors: vec![],
                    }
                })
                .collect::<Vec<_>>();

            // It's possible that there are no discussions, just a lecture
            if group.child_meetings.is_empty() {
                let mut all_meetings: Vec<Meeting> = vec![];
                main_meetings
                    .iter()
                    .for_each(|m| all_meetings.push(m.clone()));

                other_meetings
                    .iter()
                    .for_each(|x| all_meetings.push(x.clone()));

                // Just lecture = enrollment stats will be reflected properly on this meeting.
                sections.push(CourseSection {
                    subj_course_id: course_dept_id.clone(),
                    section_id: group.main_meeting[0].section_number.trim().to_string(),
                    section_code: group.main_meeting[0].sect_code.trim().to_string(),
                    needs_waitlist: group.main_meeting[0].needs_waitlist == "Y",
                    instructors: base_instructors.clone(),
                    available_seats: max(group.main_meeting[0].avail_seat, 0),
                    enrolled_ct: group.main_meeting[0].enrolled_count,
                    total_seats: group.main_meeting[0].section_capacity,
                    waitlist_ct: group.main_meeting[0].count_on_waitlist,
                    meetings: all_meetings,
                });

                continue;
            }

            // Hopefully these are discussions
            for meeting in group.child_meetings {
                let (m_type, t_m_dats) = webreg_helper::parse_meeting_type_date(meeting);
                let mut other_instructors = vec![];
                for instructor in self._get_instructor_names(&meeting.person_full_name) {
                    if base_instructors.contains(&instructor) {
                        continue;
                    }

                    other_instructors.push(instructor);
                }

                // Adding all of the main meetings to this meeting vector so we can also
                // add section-specific ones as well
                let mut all_meetings: Vec<Meeting> = vec![];
                main_meetings
                    .iter()
                    .for_each(|m| all_meetings.push(m.clone()));
                all_meetings.push(Meeting {
                    meeting_type: m_type.to_string(),
                    meeting_days: t_m_dats,
                    start_min: meeting.start_time_min,
                    start_hr: meeting.start_time_hr,
                    end_min: meeting.end_time_min,
                    end_hr: meeting.end_time_hr,
                    building: meeting.bldg_code.trim().to_string(),
                    room: meeting.room_code.trim().to_string(),
                    other_instructors,
                });

                other_meetings
                    .iter()
                    .for_each(|x| all_meetings.push(x.clone()));

                sections.push(CourseSection {
                    subj_course_id: course_dept_id.clone(),
                    section_id: meeting.section_number.trim().to_string(),
                    section_code: meeting.sect_code.trim().to_string(),
                    instructors: base_instructors.clone(),
                    available_seats: max(meeting.avail_seat, 0),
                    enrolled_ct: meeting.enrolled_count,
                    needs_waitlist: meeting.needs_waitlist == "Y",
                    total_seats: meeting.section_capacity,
                    waitlist_ct: meeting.count_on_waitlist,
                    meetings: all_meetings,
                });
            }
        }

        Ok(sections)
    }

    /// Gets all courses that are available. This searches for all courses via Webreg's menu, but
    /// then also searches each course found for specific details. This essentially calls the two
    /// functions `search_courses` and `get_course_info`.
    ///
    /// Note: This function call will make *many* API requests. Thus, searching for many classes
    /// is not recommended as you may get rate-limited.
    ///
    /// # Parameters
    /// - `filter_by`: The request filter.
    ///
    /// # Returns
    /// A result that can return one of:
    /// - A vector consisting of all courses that are available, with detailed information.
    /// - Or, the error that was encoutnered.
    pub async fn search_courses_detailed(
        &self,
        filter_by: SearchType<'_>,
    ) -> Output<'a, Vec<CourseSection>> {
        let get_zero_trim = |s: &[u8]| -> (usize, usize) {
            let start = s.iter().position(|p| *p != b'0').unwrap_or(0);
            let end = s.iter().rposition(|p| *p != b'0').unwrap_or(0);
            // "0001000" -> (3, 4)  | "0001000"[3..4] = "1"
            // "0000" -> (0, 0)     | "0000"[0..0] = ""
            // "00100100" -> (2, 6) | "00100100"[2..6] = "1001"
            (
                start,
                if start == end && start == 0 {
                    0
                } else {
                    end + 1
                },
            )
        };

        let mut ids_to_filter = vec![];
        match filter_by {
            SearchType::BySection(s) => {
                let (start, end) = get_zero_trim(s.as_bytes());
                ids_to_filter.push(&s[start..end]);
            }
            SearchType::ByMultipleSections(s) => {
                s.iter().for_each(|t| {
                    let (start, end) = get_zero_trim(t.as_bytes());
                    ids_to_filter.push(&t[start..end]);
                });
            }
            SearchType::Advanced(_) => {}
        };

        let search_res = match self.search_courses(filter_by).await {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        let mut vec: Vec<CourseSection> = vec![];
        for r in search_res {
            let req_res = self
                .get_course_info(r.subj_code.trim(), r.course_code.trim())
                .await;
            match req_res {
                Ok(r) => r.into_iter().for_each(|x| {
                    if !ids_to_filter.is_empty() {
                        let (start, end) = get_zero_trim(x.section_id.as_bytes());
                        if !ids_to_filter.contains(&&x.section_id.as_str()[start..end]) {
                            return;
                        }
                    }
                    vec.push(x);
                }),
                Err(_) => break,
            };
        }

        Ok(vec)
    }

    /// Gets all courses that are available. All this does is searches for all courses via Webreg's
    /// menu. Thus, only basic details are shown.
    ///
    /// # Parameters
    /// - `filter_by`: The request filter.
    ///
    /// # Returns
    /// A vector consisting of all courses that are available.
    pub async fn search_courses(
        &self,
        filter_by: SearchType<'_>,
    ) -> Output<'a, Vec<RawWebRegSearchResultItem>> {
        let url = match filter_by {
            SearchType::BySection(section) => Url::parse_with_params(
                WEBREG_SEARCH_SEC,
                &[("sectionid", section), ("termcode", self.term)],
            )
            .unwrap(),
            SearchType::ByMultipleSections(sections) => Url::parse_with_params(
                WEBREG_SEARCH_SEC,
                &[
                    ("sectionid", sections.join(":").as_str()),
                    ("termcode", self.term),
                ],
            )
            .unwrap(),
            SearchType::Advanced(request_filter) => {
                let subject_code = if request_filter.subjects.is_empty() {
                    "".to_string()
                } else {
                    request_filter.subjects.join(":")
                };

                let course_code = if request_filter.courses.is_empty() {
                    "".to_string()
                } else {
                    // This can probably be made significantly more efficient
                    request_filter
                        .courses
                        .iter()
                        .map(|x| x.split_whitespace().collect::<Vec<_>>())
                        .map(|course| {
                            course
                                .into_iter()
                                .map(|x| self._get_formatted_course_code(x))
                                .collect::<Vec<_>>()
                                .join(":")
                        })
                        .collect::<Vec<_>>()
                        .join(";")
                        .to_uppercase()
                };

                let department = if request_filter.departments.is_empty() {
                    "".to_string()
                } else {
                    request_filter.departments.join(":")
                };

                let professor = match request_filter.instructor {
                    Some(r) => r.to_uppercase(),
                    None => "".to_string(),
                };

                let title = match request_filter.title {
                    Some(r) => r.to_uppercase(),
                    None => "".to_string(),
                };

                let levels = if request_filter.level_filter == 0 {
                    "".to_string()
                } else {
                    // Needs to be exactly 12 digits
                    let mut s = format!("{:b}", request_filter.level_filter);
                    while s.len() < 12 {
                        s.insert(0, '0');
                    }

                    s
                };

                let days = if request_filter.days == 0 {
                    "".to_string()
                } else {
                    // Needs to be exactly 7 digits
                    let mut s = format!("{:b}", request_filter.days);
                    while s.len() < 7 {
                        s.insert(0, '0');
                    }

                    s
                };

                let time_str = {
                    if request_filter.start_time.is_none() && request_filter.end_time.is_none() {
                        "".to_string()
                    } else {
                        let start_time = match request_filter.start_time {
                            Some((h, m)) => format!("{:0>2}{:0>2}", h, m),
                            None => "".to_string(),
                        };

                        let end_time = match request_filter.end_time {
                            Some((h, m)) => format!("{:0>2}{:0>2}", h, m),
                            None => "".to_string(),
                        };

                        format!("{}:{}", start_time, end_time)
                    }
                };

                Url::parse_with_params(
                    WEBREG_SEARCH,
                    &[
                        ("subjcode", &*subject_code),
                        ("crsecode", &*course_code),
                        ("department", &*department),
                        ("professor", &*professor),
                        ("title", &*title),
                        ("levels", &*levels),
                        ("days", &*days),
                        ("timestr", &*time_str),
                        (
                            "opensection",
                            if request_filter.only_open {
                                "true"
                            } else {
                                "false"
                            },
                        ),
                        ("isbasic", "true"),
                        ("basicsearchvalue", ""),
                        ("termcode", self.term),
                        ("_", self._get_epoch_time().to_string().as_str()),
                    ],
                )
                .unwrap()
            }
        };

        self._process_get_result::<Vec<RawWebRegSearchResultItem>>(
            self.client
                .get(url)
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
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
    pub async fn send_email_to_self(&self, email_content: &str) -> bool {
        let res = self
            .client
            .post(SEND_EMAIL)
            .form(&[("actionevent", email_content), ("termcode", self.term)])
            .header(COOKIE, &self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => false,
            Ok(r) => {
                if !r.status().is_success() {
                    false
                } else {
                    r.text().await.unwrap().contains("\"YES\"")
                }
            }
        }
    }

    /// Changes the grading option for the class corresponding to the section number.
    ///
    /// # Parameters
    /// - `section_number`: The section number corresponding to the class that you want to change
    /// the grading option for.
    /// - `new_grade_opt`: The new grading option. This must either be `L` (letter),
    /// `P` (pass/no pass), or `S` (satisfactory/unsatisfactory), and is enforced via an enum.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    pub async fn change_grading_option(
        &self,
        section_number: &str,
        new_grade_opt: GradeOption,
    ) -> Output<'a, bool> {
        let new_grade_opt = match new_grade_opt {
            GradeOption::L => "L",
            GradeOption::S => "S",
            GradeOption::P => "P",
        };

        // "Slice" any zeros off of the left-most side of the string. We need to do this
        // because, when comparing section numbers in the schedule, WebReg gives us the
        // section numbers as integers; however, for the rest of the API, it's given as a
        // string.
        //
        // Essentially, this means that, while most of WebReg's API will take `"079911"` as
        // an input and as an output (e.g. see `get_course_info`), the schedule API will
        // specifically return an integer `79911`. The `get_schedule` function will simply
        // convert this integer to a string, e.g. `79911` -> `"79911"` and return that along
        // with the other parsed info for each scheduled section.
        //
        // So, we need to slice off any 0s from the input parameter `section_number` to account
        // for this.
        let mut left_idx = 0;
        for c in section_number.chars() {
            if c != '0' {
                break;
            }

            left_idx += 1;
            continue;
        }

        let poss_class = self
            .get_schedule(None)
            .await
            .unwrap_or_default()
            .into_iter()
            .find(|x| x.section_number == section_number[left_idx..]);

        if poss_class.is_none() {
            return Err("Class not found.".into());
        }

        // don't care about previous poss_class
        let poss_class = poss_class.unwrap();
        let sec_id = poss_class.section_number.to_string();
        let units = poss_class.units.to_string();

        self._process_post_response(
            self.client
                .post(CHANGE_ENROLL)
                .form(&[
                    ("section", &*sec_id),
                    ("subjCode", ""),
                    ("crseCode", ""),
                    ("unit", &*units),
                    ("grade", new_grade_opt),
                    // You don't actually need these
                    ("oldGrade", ""),
                    ("oldUnit", ""),
                    ("termcode", self.term),
                ])
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
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
    /// **WARNING:** setting this to `false` can cause issues. For example, when this is `false`,
    /// you will be able to plan courses with more units than allowed (e.g. 42 units), set the
    /// grading option to one that you are not allowed to use (e.g. S/U as an undergraduate), and
    /// only enroll in specific components of a section (e.g. just the discussion section). Some of
    /// these options can visually break WebReg (e.g. Remove/Enroll button will not appear).
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    pub async fn add_to_plan(&self, plan_options: PlanAdd<'_>, validate: bool) -> Output<'a, bool> {
        let u = plan_options.unit_count.to_string();
        let crsc_code = self._get_formatted_course_code(plan_options.course_code);

        if validate {
            // We need to call the edit endpoint first, or else we'll have issues where we don't
            // actually enroll in every component of the course.
            // Also, this can potentially return "false" due to you not being able to enroll in the
            // class, e.g. the class you're trying to plan is a major-restricted class.
            self._process_post_response(
                self.client
                    .post(PLAN_EDIT)
                    .form(&[
                        ("section", &*plan_options.section_number),
                        ("subjcode", &*plan_options.subject_code),
                        ("crsecode", &*crsc_code),
                        ("termcode", self.term),
                    ])
                    .header(COOKIE, &self.cookies)
                    .header(USER_AGENT, MY_USER_AGENT)
                    .send()
                    .await,
            )
            .await
            .unwrap_or(false);
        }

        self._process_post_response(
            self.client
                .post(PLAN_ADD)
                .form(&[
                    ("subjcode", &*plan_options.subject_code),
                    ("crsecode", &*crsc_code),
                    ("sectnum", &*plan_options.section_number),
                    ("sectcode", &*plan_options.section_code),
                    ("unit", &*u),
                    (
                        "grade",
                        match plan_options.grading_option {
                            Some(r) if r == "L" || r == "P" || r == "S" => r,
                            _ => "L",
                        },
                    ),
                    ("termcode", self.term),
                    (
                        "schedname",
                        match plan_options.schedule_name {
                            Some(r) => r,
                            None => DEFAULT_SCHEDULE_NAME,
                        },
                    ),
                ])
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await
    }

    /// Allows you to unplan a course.
    ///
    /// # Parameters
    /// - `section_num`: The section number.
    /// - `schedule_name`: The schedule name where the course should be unplanned from.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    pub async fn remove_from_plan(
        &self,
        section_num: &str,
        schedule_name: Option<&'a str>,
    ) -> Output<'a, bool> {
        self._process_post_response(
            self.client
                .post(PLAN_REMOVE)
                .form(&[
                    ("sectnum", section_num),
                    ("termcode", self.term),
                    ("schedname", schedule_name.unwrap_or(DEFAULT_SCHEDULE_NAME)),
                ])
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
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
    /// validation isn't necessary, although it is recommended. But, perhaps you just want to
    /// make one less request.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    pub async fn add_section(
        &self,
        is_enroll: bool,
        enroll_options: EnrollWaitAdd<'_>,
        validate: bool,
    ) -> Output<'a, bool> {
        let base_reg_url = if is_enroll { ENROLL_ADD } else { WAITLIST_ADD };
        let base_edit_url = if is_enroll {
            ENROLL_EDIT
        } else {
            WAITLIST_EDIT
        };

        let u = match enroll_options.unit_count {
            Some(r) => r.to_string(),
            None => "".to_string(),
        };

        if validate {
            self._process_post_response(
                self.client
                    .post(base_edit_url)
                    .form(&[
                        // These are required
                        ("section", &*enroll_options.section_number),
                        ("termcode", self.term),
                        // These are optional.
                        ("subjcode", ""),
                        ("crsecode", ""),
                    ])
                    .header(COOKIE, &self.cookies)
                    .header(USER_AGENT, MY_USER_AGENT)
                    .send()
                    .await,
            )
            .await?;
        }

        self._process_post_response(
            self.client
                .post(base_reg_url)
                .form(&[
                    // These are required
                    ("section", &*enroll_options.section_number),
                    ("termcode", self.term),
                    // These are optional.
                    ("unit", &*u),
                    (
                        "grade",
                        match enroll_options.grading_option {
                            Some(r) if r == "L" || r == "P" || r == "S" => r,
                            _ => "",
                        },
                    ),
                    ("crsecode", ""),
                    ("subjcode", ""),
                ])
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await?;

        // This will always return true
        self._process_post_response(
            self.client
                .post(PLAN_REMOVE_ALL)
                .form(&[
                    ("sectnum", &*enroll_options.section_number),
                    ("termcode", self.term),
                ])
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
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
    /// - `section_num`: The section number corresponding to the section that you want
    /// to drop.
    ///
    /// # Returns
    /// `true` if the process succeeded, or a string containing the error message from WebReg if
    /// something wrong happened.
    ///
    /// # Remarks
    /// It is a good idea to make a call to get your current schedule before you
    /// make a request here. That way, you know which classes can be dropped.
    pub async fn drop_section(&self, was_enrolled: bool, section_num: &'a str) -> Output<'a, bool> {
        let base_reg_url = if was_enrolled {
            ENROLL_DROP
        } else {
            WAILIST_DROP
        };

        self._process_post_response(
            self.client
                .post(base_reg_url)
                .form(&[
                    // These parameters are optional
                    ("subjcode", ""),
                    ("crsecode", ""),
                    // But these are required
                    ("section", section_num),
                    ("termcode", self.term),
                ])
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await
    }

    /// Pings the WebReg server. Presumably, this is the endpoint that is used to ensure that
    /// your (authenticated) session is still valid. In other words, if this isn't called, I
    /// assume that you will be logged out, rendering your cookies invalid.
    ///
    /// # Returns
    /// `true` if the ping was successful and `false` otherwise.
    pub async fn ping_server(&self) -> bool {
        let res = self
            .client
            .get(format!("{}?_={}", PING_SERVER, self._get_epoch_time()))
            .header(COOKIE, &self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => false,
            Ok(r) => {
                let text = r.text().await.unwrap_or_else(|_| {
                    json!({
                        "SESSION_OK": false
                    })
                    .to_string()
                });

                let json: Value = serde_json::from_str(&text).unwrap_or_default();
                json["SESSION_OK"].is_boolean() && json["SESSION_OK"].as_bool().unwrap()
            }
        }
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
    pub async fn rename_schedule(&self, old_name: &str, new_name: &str) -> Output<'a, bool> {
        // Can't rename your default schedule.
        if old_name == DEFAULT_SCHEDULE_NAME {
            return Err("You cannot rename the default schedule".into());
        }

        self._process_post_response(
            self.client
                .post(RENAME_SCHEDULE)
                .form(&[
                    ("termcode", self.term),
                    ("oldschedname", old_name),
                    ("newschedname", new_name),
                ])
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
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
    pub async fn remove_schedule(&self, schedule_name: &str) -> Output<'a, bool> {
        // Can't remove your default schedule.
        if schedule_name == DEFAULT_SCHEDULE_NAME {
            return Err("You cannot remove the default schedule.".into());
        }

        self._process_post_response(
            self.client
                .post(REMOVE_SCHEDULE)
                .form(&[("termcode", self.term), ("schedname", schedule_name)])
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await
    }

    /// Gets all of your schedules.
    ///
    /// # Returns
    /// A result that is either one of:
    /// - A vector of strings representing the names of the schedules
    /// - Or the error that was occurred.
    pub async fn get_schedule_list(&self) -> Output<'a, Vec<String>> {
        let url = Url::parse_with_params(ALL_SCHEDULE, &[("termcode", self.term)]).unwrap();

        self._process_get_result::<Vec<String>>(
            self.client
                .get(url)
                .header(COOKIE, &self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await
    }

    /// Processes a GET response from the resulting JSON, if any.
    ///
    /// # Parameters
    /// - `res`: The initial response.
    ///
    /// # Returns
    /// The result of processing the response.
    async fn _process_get_result<T: DeserializeOwned>(
        &self,
        res: Result<Response, Error>,
    ) -> Result<T, Cow<'a, str>> {
        match res {
            Err(e) => Err(e.to_string().into()),
            Ok(r) => {
                if !r.status().is_success() {
                    return Err(r.status().to_string().into());
                }

                let text = match r.text().await {
                    Err(e) => return Err(e.to_string().into()),
                    Ok(s) => s,
                };

                match serde_json::from_str::<T>(&text) {
                    Err(e) => Err(e.to_string().into()),
                    Ok(o) => Ok(o),
                }
            }
        }
    }

    /// Processes a POST response from the resulting JSON, if any.
    ///
    /// # Parameters
    /// - `res`: The initial response.
    ///
    /// # Returns
    /// Either one of:
    /// - `true` or `false`, depending on what WebReg returns.
    /// - or some error message if an error occurred.
    async fn _process_post_response(&self, res: Result<Response, Error>) -> Output<'a, bool> {
        match res {
            Err(e) => Err(e.to_string().into()),
            Ok(r) => {
                if !r.status().is_success() {
                    Err(r.status().to_string().into())
                } else {
                    let text = r.text().await.unwrap_or_else(|_| {
                        json!({
                            "OPS": "FAIL",
                            "REASON": ""
                        })
                        .to_string()
                    });

                    let json: Value = serde_json::from_str(&text).unwrap();
                    if json["OPS"].is_string() && json["OPS"].as_str().unwrap() == "SUCCESS" {
                        Ok(true)
                    } else {
                        let mut parsed_str = String::new();
                        let mut is_in_brace = false;
                        json["REASON"]
                            .as_str()
                            .unwrap_or("")
                            .trim()
                            .chars()
                            .for_each(|c| {
                                if c == '<' {
                                    is_in_brace = true;
                                    return;
                                }

                                if c == '>' {
                                    is_in_brace = false;
                                    return;
                                }

                                if is_in_brace {
                                    return;
                                }

                                parsed_str.push(c);
                            });

                        Err(parsed_str.into())
                    }
                }
            }
        }
    }

    /// Gets the current term.
    ///
    /// # Returns
    /// The current term.
    pub fn get_term(&self) -> &'a str {
        self.term
    }

    /// Checks if the output string represents a valid session.
    ///
    /// # Parameters
    /// - `str`: The string.
    ///
    /// # Returns
    /// `true` if the string doesn't contain signs that we have an invalid session.
    #[inline(always)]
    fn _internal_is_valid(&self, str: &str) -> bool {
        !str.contains("Skip to main content")
    }

    /// Gets the current epoch time.
    ///
    /// # Returns
    /// The current time.
    fn _get_epoch_time(&self) -> u128 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    /// Gets the formatted course code so that it can be recognized by WebReg's internal API.
    ///
    /// # Parameters
    /// - `course_code`: The course code, e.g. if you have the course `CSE 110`, you would put
    /// `110`.
    ///
    /// # Returns
    /// The formatted course code for WebReg.
    #[inline(always)]
    fn _get_formatted_course_code(&self, course_code: &str) -> String {
        // If the course code only has 1 digit (excluding any letters), then we need to prepend 2
        // spaces to the course code.
        //
        // If the course code has 2 digits (excluding any letters), then we need to prepend 1
        // space to the course code.
        //
        // Otherwise, don't need to prepend any spaces to the course code.
        //
        // For now, assume that no digits will ever appear *after* the letters. Weird thing is that
        // WebReg uses '+' to offset the course code but spaces are accepted.
        match course_code.chars().filter(|x| x.is_ascii_digit()).count() {
            1 => format!("  {}", course_code),
            2 => format!(" {}", course_code),
            _ => course_code.to_string(),
        }
    }

    /// Gets the instructor's names.
    ///
    /// # Parameters
    /// - `instructor_name`: The raw name.
    ///
    /// # Returns
    /// The parsed instructor's names, as a vector.
    fn _get_instructor_names(&self, instructor_name: &str) -> Vec<String> {
        // The instructor string is in the form
        // name1    ;pid1:name2      ;pid2:...:nameN      ;pidN
        instructor_name
            .split(':')
            .map(|x| {
                if x.contains(';') {
                    x.split_once(';').unwrap().0.trim().to_string()
                } else {
                    x.trim().to_string()
                }
            })
            .collect()
    }

    /// Removes duplicate names from the list of instructors that are given.
    ///
    /// # Parameters
    /// - `instructors`: An iterator of instructors, potentially with duplicates.
    ///
    /// # Returns
    /// A vector of instructors, with no duplicates.
    fn _get_all_instructors<I>(&self, instructors: I) -> Vec<String>
    where
        I: Iterator<Item = String>,
    {
        let mut all_inst = instructors.collect::<Vec<_>>();
        all_inst.sort();
        all_inst.dedup();
        all_inst
    }
}

// Helper structure for organizing meetings. Only used once for now.
#[derive(Debug)]
struct GroupedSection<'a, T> {
    main_meeting: Vec<&'a T>,
    child_meetings: Vec<&'a T>,
    other_special_meetings: Vec<&'a T>,
}

/// Use this struct to add more information regarding the section that you want to enroll/waitlist
/// in.
///
/// An example of this struct in use can be seen below (taken from the README):
/// ```rs
/// let add_res = w
///     .add_section(
///         true,
///         EnrollWaitAdd {
///             section_number: "078616",
///             // Use default grade option
///             grading_option: None,
///             // Use default unit count
///             unit_count: None,
///         },
///         true,
///     )
///     .await;
/// ```
pub struct EnrollWaitAdd<'a> {
    /// The section number. For example, `0123123`.
    pub section_number: &'a str,
    /// The grading option. Can either be L, P, or S.
    /// If None is specified, this uses the default option.
    pub grading_option: Option<&'a str>,
    /// The number of units. If none is specified, this
    /// uses the default unit count.
    pub unit_count: Option<u8>,
}

/// Use this struct to add more information regarding the course that you want to plan.
///
/// An example of this struct in use can be seen below (taken from the README):
/// ```rs
/// let res = w.add_to_plan(PlanAdd {
///     subject_code: "CSE",
///     course_code: "100",
///     section_number: "079911",
///     section_code: "A01",
///     // Using S/U grading.
///     grading_option: Some("S"),
///     // Put in default schedule
///     schedule_name: None,
///     unit_count: 4
/// }, true).await;
/// match res {
///     Ok(o) => println!("{}", if o { "Successful" } else { "Unsuccessful" }),
///     Err(e) => eprintln!("{}", e),
/// };
/// ```
pub struct PlanAdd<'a> {
    /// The subject code. For example, `CSE`.
    pub subject_code: &'a str,
    /// The course code. For example, `12`.
    pub course_code: &'a str,
    /// The section number. For example, `0123123`.
    pub section_number: &'a str,
    /// The section code. For example `A00`.
    pub section_code: &'a str,
    /// The grading option. Can either be L, P, or S.
    pub grading_option: Option<&'a str>,
    /// The schedule name.
    pub schedule_name: Option<&'a str>,
    /// The number of units.
    pub unit_count: u8,
}

/// Used to construct search requests for the `search_courses` function.
pub struct SearchRequestBuilder<'a> {
    subjects: Vec<&'a str>,
    courses: Vec<&'a str>,
    departments: Vec<&'a str>,
    instructor: Option<&'a str>,
    title: Option<&'a str>,
    level_filter: u32,
    days: u32,
    start_time: Option<(u32, u32)>,
    end_time: Option<(u32, u32)>,
    only_open: bool,
}

impl<'a> SearchRequestBuilder<'a> {
    /// Creates a new instance of the `SearchRequestBuilder`, which is used to search for specific
    /// courses.
    ///
    /// # Returns
    /// The empty `SearchRequestBuilder`.
    pub fn new() -> Self {
        Self {
            subjects: vec![],
            courses: vec![],
            departments: vec![],
            instructor: None,
            title: None,
            level_filter: 0,
            days: 0,
            start_time: None,
            end_time: None,
            only_open: false,
        }
    }

    /// Adds a subject to this search request. Valid search requests are uppercase and at most
    /// 4 characters long. Some examples include `MATH` or `CSE`.
    ///
    /// # Parameters
    /// - `subject`: The subject.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn add_subject(mut self, subject: &'a str) -> Self {
        if subject != subject.to_uppercase() || subject.len() > 4 {
            return self;
        }

        self.subjects.push(subject);
        self
    }

    /// Adds a course (either a subject code, course code, or both) to the search request. Some
    /// examples include `20E`, `math 20d`, `101`, `CSE`.
    ///
    /// # Parameters
    /// - `course`: The course.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn add_course(mut self, course: &'a str) -> Self {
        self.courses.push(course);
        self
    }

    /// Adds a department to the search request. Valid search requests are uppercase and at most 4
    /// characters long. Some examples include `MATH` or `CSE`.
    ///
    /// # Parameters
    /// - `department`: The department.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn add_department(mut self, department: &'a str) -> Self {
        if department != department.to_uppercase() || department.len() > 4 {
            return self;
        }

        self.departments.push(department);
        self
    }

    /// Sets the instructor to the specified instructor.
    ///
    /// # Parameters
    /// - `instructor`: The instructor. This should be formatted in `Last Name, First Name` form.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn set_instructor(mut self, instructor: &'a str) -> Self {
        self.instructor = Some(instructor);
        self
    }

    /// Sets the course title to the specified title. Some examples could be `differential equ`,
    /// `data structures`, `algorithms`, and so on.
    ///
    /// # Parameters
    /// - `title`: The title of the course.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn set_title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Restrict search results to to the specified filter. This can be applied multiple times.
    ///
    /// # Parameters
    /// - `filter`: The filter.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn filter_courses_by(mut self, filter: CourseLevelFilter) -> Self {
        self.level_filter |= match filter {
            CourseLevelFilter::LowerDivision => 1 << 11,
            CourseLevelFilter::FreshmenSeminar => 1 << 10,
            CourseLevelFilter::LowerDivisionIndepStudy => 1 << 9,
            CourseLevelFilter::UpperDivision => 1 << 8,
            CourseLevelFilter::Apprenticeship => 1 << 7,
            CourseLevelFilter::UpperDivisionIndepStudy => 1 << 6,
            CourseLevelFilter::Graduate => 1 << 5,
            CourseLevelFilter::GraduateIndepStudy => 1 << 4,
            CourseLevelFilter::GraduateResearch => 1 << 3,
            CourseLevelFilter::Lvl300 => 1 << 2,
            CourseLevelFilter::Lvl400 => 1 << 1,
            CourseLevelFilter::Lvl500 => 1 << 0,
        };

        self
    }

    /// Only shows courses based on the specified day(s).
    ///
    /// # Parameters
    /// - `day`: The day.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn apply_days(mut self, day: DayOfWeek) -> Self {
        let day = match day {
            DayOfWeek::Monday => 1,
            DayOfWeek::Tuesday => 2,
            DayOfWeek::Wednesday => 3,
            DayOfWeek::Thursday => 4,
            DayOfWeek::Friday => 5,
            DayOfWeek::Saturday => 6,
            DayOfWeek::Sunday => 7,
        };

        self.days |= 1 << (7 - day);
        self
    }

    /// Sets the start time to the specified time.
    ///
    /// # Parameters
    /// - `hour`: The hour. This should be between 0 and 23, inclusive.
    /// - `min`: The minute. This should be between 0 and 59, inclusive.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn set_start_time(mut self, hour: u32, min: u32) -> Self {
        if hour > 23 || min > 59 {
            return self;
        }

        self.start_time = Some((hour, min));
        self
    }

    /// Sets the end time to the specified time.
    ///
    /// # Parameters
    /// - `hour`: The hour. This should be between 0 and 23, inclusive.
    /// - `min`: The minute. This should be between 0 and 59, inclusive.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn set_end_time(mut self, hour: u32, min: u32) -> Self {
        if hour > 23 || min > 59 {
            return self;
        }

        self.end_time = Some((hour, min));
        self
    }

    /// Whether to only show sections with open seats.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn only_allow_open(mut self) -> Self {
        self.only_open = true;
        self
    }
}

/// The day of week enum, which designates what days you want
/// to filter specific sections by.
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// The course level filter enum, which can be used to filter
/// specific sections by.
pub enum CourseLevelFilter {
    /// Level 1-99 courses.
    LowerDivision,
    /// Level 87, 90 courses.
    FreshmenSeminar,
    /// Level 99 courses.
    LowerDivisionIndepStudy,
    /// Level 100-198 courses
    UpperDivision,
    /// Level 195 courses
    Apprenticeship,
    /// Level 199 courses
    UpperDivisionIndepStudy,
    /// Level 200-297 courses
    Graduate,
    /// Level 298 courses
    GraduateIndepStudy,
    /// Level 299 courses
    GraduateResearch,
    /// Level 300+ courses
    Lvl300,
    /// Level 400+ courses
    Lvl400,
    /// Level 500+ courses
    Lvl500,
}

/// Lets you choose how you want to search for a course.
pub enum SearchType<'a> {
    /// Searches for a course by section ID.
    BySection(&'a str),

    /// Searches for a course by more than one section ID.
    ByMultipleSections(&'a [&'a str]),

    /// Searches for a (set of) course(s) by multiple specifications.
    Advanced(&'a SearchRequestBuilder<'a>),
}

/// The possible grading options.
pub enum GradeOption {
    /// S/U grading (Satisfactory/Unsatisfactory) option.
    S,

    /// P/NP grading (Pass/No Pass) option.
    P,

    /// Letter grading option.
    L,
}
