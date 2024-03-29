use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// One possible result you can get by searching for a particular course.
#[derive(Debug, Serialize, Deserialize)]
pub struct RawWebRegSearchResultItem {
    /// The maximum number of units you can get.
    #[serde(rename = "UNIT_TO")]
    max_units: f32,

    /// The subject code. For example, `CSE` or `MATH` are both possible option.
    #[serde(rename = "SUBJ_CODE")]
    pub subj_code: String,

    /// The course title. For example, `Abstract Algebra II`.
    #[serde(rename = "CRSE_TITLE")]
    pub course_title: String,

    /// The minimum number of units you can get.
    #[serde(rename = "UNIT_FROM")]
    min_units: f32,

    /// The course code. For example, `100B`.
    #[serde(rename = "CRSE_CODE")]
    pub course_code: String,
}

impl Display for RawWebRegSearchResultItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{} {}] {} ({})",
            self.subj_code.trim(),
            self.course_code.trim(),
            self.course_title.trim(),
            self.max_units
        )
    }
}

/// A meeting. Note that this doesn't represent a class by itself, but rather a "piece" of that
/// class. For example, one `WebRegMeeting` can represent a discussion while another can
/// represent a lecture.
#[derive(Debug, Serialize, Deserialize)]
pub struct RawWebRegMeeting {
    /// The hour part of the end time. For example, if this meeting ends at 11:50 AM, then
    /// this would be `11`.
    #[serde(rename = "END_HH_TIME")]
    pub end_time_hr: i16,

    /// The minutes part of the end time. For example, if this meeting ends at 11:50 AM, then
    /// this would be `50`.
    #[serde(rename = "END_MM_TIME")]
    pub end_time_min: i16,

    /// The section capacity. For example, if this section has a limit of 196, then this would be
    /// `196`.
    #[serde(rename = "SCTN_CPCTY_QTY")]
    pub section_capacity: i64,

    /// The number of students enrolled in this section.
    #[serde(rename = "SCTN_ENRLT_QTY")]
    pub enrolled_count: i64,

    /// The section ID. Each section has a unique number identifier.
    #[serde(rename = "SECTION_NUMBER")]
    pub section_id: String,

    /// The number of students currently on the waitlist.
    #[serde(rename = "COUNT_ON_WAITLIST")]
    pub count_on_waitlist: i64,

    /// The room code. For example, if the meeting is in CENTR 119, then this would be `119`.
    #[serde(rename = "ROOM_CODE")]
    pub room_code: String,

    /// The minute part of the meeting start time. For example, if this meeting starts at 11:00 AM,
    /// then this would be `0`.
    #[serde(rename = "BEGIN_MM_TIME")]
    pub start_time_min: i16,

    /// The hours part of the start time. For example, if this meeting starts at 11:00 AM, then
    /// this would be `11`.
    #[serde(rename = "BEGIN_HH_TIME")]
    pub start_time_hr: i16,

    /// The days that this meeting will take place. This string will only consist of the following:
    /// - `1`: Monday
    /// - `2`: Tuesday
    /// - `3`: Wednesday
    /// - `4`: Thursday
    /// - `5`: Friday
    ///
    /// For example, if a class is meeting MWF, this would be `135`.
    #[serde(rename = "DAY_CODE")]
    pub day_code: String,

    /// The instructor(s).
    #[serde(rename = "PERSON_FULL_NAME")]
    pub person_full_name: String,

    /// Special meeting type, if any. If this is a normal meeting, this will be a string with a
    /// two spaces.
    #[serde(rename = "FK_SPM_SPCL_MTG_CD")]
    pub special_meeting: String,

    /// The building code. For example, if the meeting will take place at Center Hall, this would
    /// be `CENTR`.
    #[serde(rename = "BLDG_CODE")]
    pub bldg_code: String,

    /// The meeting type. See https://registrar.ucsd.edu/StudentLink/instr_codes.html. Note that
    /// this will improperly record final exams, midterms, and other special events as lectures.
    /// So, you need to check `special_meeting` also.
    #[serde(rename = "FK_CDI_INSTR_TYPE")]
    pub meeting_type: String,

    /// The section code. For example, this could be `A00` or `B01`.
    #[serde(rename = "SECT_CODE")]
    pub sect_code: String,

    /// The number of available seats.
    #[serde(rename = "AVAIL_SEAT")]
    pub avail_seat: i64,

    /// The date that this meeting starts. Note that this (`start_date`) and `section_start_date`
    /// will have different dates if the meeting that this `WebRegEvent` represents is a one-day
    /// event (e.g. final exam).
    #[serde(rename = "START_DATE")]
    pub start_date: String,

    /// The date that this section officially starts.
    #[serde(rename = "SECTION_START_DATE")]
    pub section_start_date: String,

    /// How this particular entry is displayed. From my understanding, it looks like:
    /// - `AC`: A section that can be enrolled or planned.
    /// - `NC`: A section that cannot be enrolled or planned (see CSE 8A Discussions).
    /// - `CA`: Canceled.
    #[serde(rename = "FK_SST_SCTN_STATCD")]
    pub display_type: String,

    /// No idea what this does, but I'm assuming this tells you if the section
    /// is visible on WebReg.
    /// - `" "` (an empty space) or `"Y"` if it is visible, and
    /// - `"N"` if it is not visible.
    #[serde(rename = "PRINT_FLAG")]
    pub print_flag: String,
}

impl RawWebRegMeeting {
    /// Whether the meeting is visible on WebReg.
    ///
    /// I don't know if this actually works.
    ///
    /// # Returns
    /// `true` if the meeting is visible on WebReg, and `false` otherwise.
    pub fn is_visible(&self) -> bool {
        self.print_flag.as_str() == "Y" || self.print_flag == " "
    }
}

/// A meeting that you have enrolled in. Note that this doesn't represent a class by itself, but
/// rather a "piece" of that class. For example, one `ScheduledMeeting` can represent a discussion
/// while another can represent a lecture. Additionally, each `ScheduledMeeting` can only represent
/// one meeting per week (so, for example, a MWF lecture would have 3 entries).
#[derive(Serialize, Deserialize, Debug)]
pub struct RawScheduledMeeting {
    /// The section ID. Each section has a unique number identifier.
    #[serde(rename = "SECTION_HEAD")]
    pub section_id: i64,

    /// Number of units that this class is being taken for (e.g. 4.00)
    #[serde(rename = "SECT_CREDIT_HRS")]
    pub sect_credit_hrs: f32,

    /// The minute part of the meeting start time. For example, if this meeting starts at 11:00 AM,
    /// then this would be `0`.
    #[serde(rename = "BEGIN_MM_TIME")]
    pub start_time_min: i16,

    /// The hours part of the start time. For example, if this meeting starts at 11:00 AM, then
    /// this would be `11`.
    #[serde(rename = "BEGIN_HH_TIME")]
    pub start_time_hr: i16,

    /// The hour part of the end time. For example, if this meeting ends at 11:50 AM, then
    /// this would be `11`.
    #[serde(rename = "END_HH_TIME")]
    pub end_time_hr: i16,

    /// The minutes part of the end time. For example, if this meeting ends at 11:50 AM, then
    /// this would be `50`.
    #[serde(rename = "END_MM_TIME")]
    pub end_time_min: i16,

    /// The subject code. For example, `CSE` or `MATH` are both possible option.
    #[serde(rename = "SUBJ_CODE")]
    pub subj_code: String,

    /// The room code. For example, if the meeting is in CENTR 119, then this would be `119`.
    #[serde(rename = "ROOM_CODE")]
    pub room_code: String,

    /// The course title. For example, `Abstract Algebra II`.
    #[serde(rename = "CRSE_TITLE")]
    pub course_title: String,

    /// The grading option. Some common options are `P/NP` or `L`, the former being pass/no pass
    /// and the latter being letter.
    #[serde(rename = "GRADE_OPTION")]
    pub grade_option: String,

    /// The day that this meeting starts. For lectures, this will usually be the first day of the
    /// quarter; for midterms and finals, these will be given different dates.
    #[serde(rename = "START_DATE")]
    pub start_date: String,

    /// The course code. For example, `100B`.
    #[serde(rename = "CRSE_CODE")]
    pub course_code: String,

    /// The day code. Unlike in `WebRegMeeting`, this stores at most 1 number.
    #[serde(rename = "DAY_CODE")]
    pub day_code: String,

    /// The professor teaching this course.
    #[serde(rename = "PERSON_FULL_NAME")]
    pub person_full_name: String,

    /// Special meeting type, if any. If this is a normal meeting, this will be a string with a
    /// two spaces. Note that
    #[serde(rename = "FK_SPM_SPCL_MTG_CD")]
    pub special_meeting: String,

    /// The meeting type. See https://registrar.ucsd.edu/StudentLink/instr_codes.html. Note that
    /// this will properly show the event type.
    #[serde(rename = "FK_CDI_INSTR_TYPE")]
    pub meeting_type: String,

    /// The building code. For example, if the meeting will take place at Center Hall, this would
    /// be `CENTR`.
    #[serde(rename = "BLDG_CODE")]
    pub bldg_code: String,

    /// The current enrollment status. This can be one of:
    /// - `EN`: Enrolled
    /// - `WT`: Waitlisted
    /// - `PL`: Planned
    #[serde(rename = "ENROLL_STATUS")]
    pub enroll_status: String,

    /// The section code. For example, this could be `A00` or `B01`.
    #[serde(rename = "SECT_CODE")]
    pub sect_code: String,

    /// The maximum number of students that can enroll in this section. Note that this is an
    /// `Option` type; this is because this value won't exist if you can't directly enroll in the
    /// section (e.g. you can't directly enroll in a lecture but you can directly enroll in a
    /// lecture + discussion).
    #[serde(rename = "SCTN_CPCTY_QTY")]
    pub section_capacity: Option<i64>,

    /// The number of students enrolled in this section. See `section_capacity` for information.
    #[serde(rename = "SCTN_ENRLT_QTY")]
    pub enrolled_count: Option<i64>,

    /// The number of students currently on the waitlist.
    #[serde(rename = "COUNT_ON_WAITLIST")]
    pub count_on_waitlist: Option<i64>,

    /// Your waitlist position. This will either be an empty string if there is no waitlist,
    /// or your waitlist position if you are on the waitlist.
    #[serde(rename = "WT_POS")]
    pub waitlist_pos: String,
}

/// An enum that represents a prerequisite type. Generally, WebReg displays prerequisites as either
/// a course requirement or a test requirement.
///
/// If we're working with a course requirement, then WebReg will categorize each course requirement
/// by its `PREREQ_SEQ_ID`. For example, if course prerequisite A and B has PREREQ_SEQ_ID 1 and
/// course prerequisite C has PREREQ_SEQ_ID 2, then this means that the prerequisites for this
/// course is
/// - one of A or B, and
/// - C.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "TYPE")]
pub enum RawPrerequisite {
    /// Whether the prerequisite is a test/exam.
    #[serde(rename = "TEST")]
    Test(RawTestPrerequisite),

    /// Whether the prerequisite is a course.
    #[serde(rename = "COURSE")]
    Course(RawCoursePrerequisite),
}

// Don't use inline struct in enum since that makes pattern matching unnecessary later.
#[derive(Serialize, Deserialize, Debug)]
pub struct RawTestPrerequisite {
    /// The name of the test/exam.
    #[serde(rename = "TEST_TITLE")]
    pub test_title: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawCoursePrerequisite {
    /// The subject code. For example, `CSE` or `MATH` are both possible option.
    #[serde(rename = "SUBJECT_CODE")]
    pub subject_code: String,

    /// The group that this prerequisite is in. For example, if there are two prerequisites
    /// with ID 1, then this means you just need ONE of those two prerequisites.
    #[serde(rename = "PREREQ_SEQ_ID")]
    pub prereq_seq_id: String,

    /// The name of the course.
    #[serde(rename = "CRSE_TITLE")]
    pub course_title: String,

    /// The course code. For example, `100A` is a possible option.
    #[serde(rename = "COURSE_CODE")]
    pub course_code: String,

    // This always seem to be 450 or 600 or some multiple of 50.
    #[serde(rename = "GRADE_SEQ_ID")]
    pub grade_seq_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct RawEvent {
    /// The location of the event.
    #[serde(rename = "LOCATION")]
    pub location: String,

    /// The start time. Guaranteed to be length 4, where the first
    /// two characters is the hour and the last two are minutes.
    #[serde(rename = "START_TIME")]
    pub start_time: String,

    /// The end time. Guaranteed to be length 4, where the first
    /// two characters is the hour and the last two are minutes.
    #[serde(rename = "END_TIME")]
    pub end_time: String,

    /// A description of the event. AKA the name of the event.
    #[serde(rename = "DESCRIPTION")]
    pub description: String,

    /// The days that this event will occur, represented as a binary
    /// string. This is guaranteed to be length 7, where `0` means the
    /// day is not selected and `1` means the day is selected. The first
    /// bit will always be Monday, the second will always be Tuesday,
    /// and the last bit will always be Sunday. In other words, the binary
    /// string is formatted like so:
    /// ```txt
    ///     MON TUE WED THU FRI SAT SUN
    /// ```
    /// So, for example, if we have `1010111`, then this means that Monday,
    /// Wednesday, Friday, Saturday, and Sunday are selected.
    #[serde(rename = "DAYS")]
    pub days: String,

    /// The timestamp, representing when the event was created. Use this
    /// value to remove an event.
    #[serde(rename = "TIME_STAMP")]
    pub time_stamp: String,
}

// For those interested, a department and a subject are NOT the
// same things, despite having many similar elements.
//
// The best way to think about it is: a department can have
// multiple *subjects*.

#[derive(Serialize, Deserialize)]
pub struct RawSubjectElement {
    /// The subject description. For example,
    /// `Mathematics`.
    #[serde(rename = "LONG_DESC")]
    pub long_desc: String,

    /// The subject code. For example, `MATH`.
    #[serde(rename = "SUBJECT_CODE")]
    pub subject_code: String,
}

#[derive(Serialize, Deserialize)]
pub struct RawDepartmentElement {
    /// The department code. For example, `MATH`.
    #[serde(rename = "DEP_CODE")]
    pub dep_code: String,

    /// The department description. For example,
    /// `Mathematics`.
    #[serde(rename = "DEP_DESC")]
    pub dep_desc: String,
}

#[derive(Serialize, Deserialize)]
pub struct RawTermListItem {
    /// The term description (e.g., Fall 2023).
    #[serde(rename = "termDesc")]
    pub term_desc: String,
    /// The sequence ID.
    #[serde(rename = "seqId")]
    pub seq_id: i64,
    /// The term code (e.g., FA23).
    #[serde(rename = "termCode")]
    pub term_code: String,
}

#[derive(Serialize, Deserialize)]
pub struct RawCourseTextItem {
    /// This partitioning of the course text information.
    #[serde(rename = "TEXT")]
    pub text: String,
    /// The course code, where the subject and number is separated by a colon (e.g., `CSE:100`).
    #[serde(rename = "SUBJCRSE")]
    pub subj_crse: String,
}

#[derive(Serialize, Deserialize)]
pub struct RawSectionTextItem {
    /// The course section number (e.g., `123456`).
    #[serde(rename = "SECTNUM")]
    pub sectnum: String,

    /// This partitioning of the subject text information.
    #[serde(rename = "TEXT")]
    pub text: String,
}
