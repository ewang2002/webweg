use std::borrow::Cow;

use serde::Serialize;

/// A section, which consists of a lecture, usually a discussion, and usually a final.
#[derive(Debug, Clone, Serialize)]
pub struct CourseSection {
    /// The subject, course ID. For example, `CSE 100`.
    pub subj_course_id: String,
    /// The section ID. For example, `079912`.
    pub section_id: String,
    /// The section code. For example, `B01`.
    pub section_code: String,
    /// All instructors (i.e., all of the instructors that appear in the `meetings`).
    pub all_instructors: Vec<String>,
    /// The number of available seats. For example, suppose a section had 30 seats
    /// total and there are 5 people enrolled. Then, this will be `25`.
    pub available_seats: i64,
    /// The number of students enrolled in this section. For example, suppose a
    /// section had 30 seats total and there are 5 people enrolled. Then, this will
    /// be `5`.
    pub enrolled_ct: i64,
    /// The total number of seats.
    pub total_seats: i64,
    /// The waitlist count.
    pub waitlist_ct: i64,
    /// All meetings.
    pub meetings: Vec<Meeting>,
    /// Whether you need to waitlist this.
    pub needs_waitlist: bool,
}

impl CourseSection {
    /// Checks if this section has any seats left.
    ///
    /// This function should be used because, sometimes, WebReg will say that
    /// there are some seats available; however, in reality, no seats are
    /// available and, usually, there is still a waitlist.
    ///
    /// # Returns
    /// `true` if there are seats and `false` otherwise.
    pub fn has_seats(&self) -> bool {
        self.available_seats > 0 && self.waitlist_ct == 0
    }
}

impl ToString for CourseSection {
    fn to_string(&self) -> String {
        let mut s = format!(
            "[{}] [{} / {}] {} - Avail.: {}, Enroll.: {}, Total: {} (WL: {}) [{}]\n",
            self.subj_course_id,
            self.section_code,
            self.section_id,
            self.all_instructors.join(" & "),
            self.available_seats,
            self.enrolled_ct,
            self.total_seats,
            self.waitlist_ct,
            if self.has_seats() { "E" } else { "W" }
        );

        for meeting in &self.meetings {
            s.push_str(&*meeting.to_string());
            s.push('\n');
        }

        s
    }
}

/// A meeting. Usually represents a lecture, final exam, discussion, and more.
#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
pub struct Meeting {
    /// The meeting type. For example, this can be `LE`, `FI`, `DI`, etc.
    pub meeting_type: String,
    /// The meeting day(s). This is an enum that represents either a reoccurring meeting
    /// or one-time meeting.
    #[serde(rename = "meeting_days")]
    pub meeting_days: MeetingDay,
    /// The start hour. For example, if the meeting starts at 14:15, this would be `14`.
    pub start_hr: i16,
    /// The start minute. For example, if the meeting starts at 14:15, this would be `15`.
    pub start_min: i16,
    /// The end hour. For example, if the meeting ends at 15:05, this would be `15`.
    pub end_hr: i16,
    /// The end minute. For example, if the meeting ends at 15:05, this would be `5`.
    pub end_min: i16,
    /// The building where this meeting will occur. For example, if the meeting is held in
    /// `CENTR 115`, then this would be `CENTR`.
    pub building: String,
    /// The room number where this meeting will occur. For example, if the meeting is held in
    /// `CENTR 115`, then this would be `115`.
    pub room: String,
    /// The instructors assigned to this meeting.
    pub instructors: Vec<String>,
}

/// An enum that represents the meeting days for a section meeting.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum MeetingDay {
    /// The meeting is repeated. In this case, each element in the vector will be one of the
    /// following: `M`, `Tu`, `W`, `Th`, `F`, `Sa`, or `Su`.
    Repeated(Vec<String>),
    /// The meeting occurs once. In this case, the string will just be the date representation
    /// in the form `YYYY-MM-DD`.
    OneTime(String),
    /// There is no meeting.
    None,
}

impl Meeting {
    /// Returns a flat string representation of this `Meeting`. One example of a flat string might
    /// look like
    /// ```txt
    /// MWF LE 13:00 - 13:50 CENTR 115 .. OTHER_INSTRUCTOR_1 & ... & OTHER_INSTRUCTOR_n
    /// ```
    ///
    /// This flat string is generally useful when needing to store meeting data in a CSV or TSV
    /// file.
    ///
    /// # Returns
    /// A flat string representation of this `Meeting`. Useful for CSV files.
    pub fn to_flat_str(&self) -> String {
        let mut s = String::new();
        s.push_str(&match &self.meeting_days {
            MeetingDay::Repeated(r) => r.join(""),
            MeetingDay::OneTime(r) => r.to_string(),
            MeetingDay::None => "N/A".to_string(),
        });

        s.push(' ');
        s.push_str(self.meeting_type.as_str());
        s.push(' ');
        s.push_str(&format!(
            "{}:{:02}-{}:{:02}",
            self.start_hr, self.start_min, self.end_hr, self.end_min
        ));

        s.push(' ');
        s.push_str(&format!("{} {}", self.building, self.room));

        s.push_str("..");
        s.push_str(&self.instructors.join(" & "));

        s
    }
}

impl ToString for Meeting {
    fn to_string(&self) -> String {
        let meeting_days_display: Cow<'_, str> = match &self.meeting_days {
            MeetingDay::Repeated(r) => r.join("").into(),
            MeetingDay::OneTime(r) => r.into(),
            MeetingDay::None => "N/A".into(),
        };

        let time_range = format!(
            "{}:{:02} - {}:{:02}",
            self.start_hr, self.start_min, self.end_hr, self.end_min
        );
        format!(
            "\t[{}] {} at {} in {} {} [{}]",
            self.meeting_type,
            meeting_days_display,
            time_range,
            self.building,
            self.room,
            self.instructors.join(" & ")
        )
    }
}

/// A section that is currently in your schedule. Note that this can either be a course that you
/// are enrolled in, waitlisted for, or planned.
#[derive(Debug, Clone, Serialize)]
pub struct ScheduledSection {
    /// The section ID, for example `79903`.
    pub section_id: String,
    /// The subject code. For example, if this represents `CSE 100`, then this would be `CSE`.
    pub subject_code: String,
    /// The subject code. For example, if this represents `CSE 100`, then this would be `100`.
    pub course_code: String,
    /// The course title, for example `Advanced Data Structure`.
    pub course_title: String,
    /// The section code, for example `A01`.
    pub section_code: String,
    /// The section capacity (maximum number of people that can enroll in this section).
    pub section_capacity: i64,
    /// The number of people enrolled in this section.
    pub enrolled_count: i64,
    /// The number of available seats left.
    pub available_seats: i64,
    /// The grading option. This can be one of `L`, `P`, or `S`.
    pub grade_option: String,
    /// All instructors that appear in all of the meetings.
    pub all_instructors: Vec<String>,
    /// The number of units that you are taking this course for.
    pub units: f32,
    /// Your enrollment status.
    #[serde(rename = "enrolled_status")]
    pub enrolled_status: EnrollmentStatus,
    /// The number of people on the waitlist.
    pub waitlist_ct: i64,
    /// All relevant meetings for this section.
    pub meetings: Vec<Meeting>,
}

impl ToString for ScheduledSection {
    fn to_string(&self) -> String {
        let status: Cow<'_, str> = match self.enrolled_status {
            EnrollmentStatus::Enrolled => "Enrolled".into(),
            EnrollmentStatus::Waitlist(r) => {
                format!("Waitlisted {}/{}", r, self.waitlist_ct).into()
            }
            EnrollmentStatus::Planned => "Planned".into(),
            EnrollmentStatus::Unknown => "Unknown".into(),
        };

        let mut s = format!(
            "[{} / {}] {} ({} {}) with {} - {} ({} Units, {} Grading, Avail.: {}, Enroll.: {}, Total: {})\n",
            self.section_code,
            self.section_id,
            self.course_title,
            self.subject_code,
            self.course_code,
            self.all_instructors.join(" & "),
            status,
            self.units,
            self.grade_option,
            self.available_seats,
            self.enrolled_count,
            self.section_capacity
        );

        for meeting in &self.meetings {
            s.push_str(&*meeting.to_string());
            s.push('\n');
        }

        s
    }
}

/// An enum that represents your enrollment status.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum EnrollmentStatus {
    Enrolled,
    Waitlist(i64),
    Planned,
    Unknown,
}
