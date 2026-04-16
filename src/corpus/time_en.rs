use crate::corpus::*;
use crate::ranking::train::TrainingCorpus;
use crate::Grain;
use chrono::{FixedOffset, TimeZone};

/// English time training corpus, ported from Duckling/Time/EN/Corpus.hs.
pub fn corpus() -> TrainingCorpus {
    let context = crate::resolve::Context::new(
        FixedOffset::west_opt(2 * 3600)
            .unwrap()
            .with_ymd_and_hms(2013, 2, 12, 4, 30, 0)
            .unwrap(),
        crate::locale::Locale::new(crate::locale::Lang::EN, None),
    );
    build_corpus(
        context,
        vec![
            // -- allExamples --
            examples(
                datetime(2013, 2, 12, 4, 30, 0, Grain::Second),
                vec!["now", "right now", "just now", "at the moment", "ATM"],
            ),
            examples(
                datetime(2013, 2, 12, 0, 0, 0, Grain::Day),
                vec!["today", "at this time"],
            ),
            examples(datetime(2013, 2, 1, 0, 0, 0, Grain::Month), vec!["2/2013"]),
            examples(datetime(2014, 1, 1, 0, 0, 0, Grain::Year), vec!["in 2014"]),
            examples(
                datetime(2013, 2, 11, 0, 0, 0, Grain::Day),
                vec!["yesterday"],
            ),
            examples(
                datetime(2013, 2, 13, 0, 0, 0, Grain::Day),
                vec!["tomorrow", "tomorrows"],
            ),
            examples(
                datetime(2013, 2, 18, 0, 0, 0, Grain::Day),
                vec![
                    "monday",
                    "mon.",
                    "this monday",
                    "Monday, Feb 18",
                    "Mon, February 18",
                ],
            ),
            examples(
                datetime(2013, 2, 19, 0, 0, 0, Grain::Day),
                vec!["tuesday", "Tuesday the 19th", "Tuesday 19th"],
            ),
            examples(datetime(2013, 8, 15, 0, 0, 0, Grain::Day), vec!["Thu 15th"]),
            examples(
                datetime(2013, 2, 14, 0, 0, 0, Grain::Day),
                vec!["thursday", "thu", "thu."],
            ),
            examples(
                datetime(2013, 2, 15, 0, 0, 0, Grain::Day),
                vec!["friday", "fri", "fri."],
            ),
            examples(
                datetime(2013, 2, 16, 0, 0, 0, Grain::Day),
                vec!["saturday", "sat", "sat."],
            ),
            examples(
                datetime(2013, 2, 17, 0, 0, 0, Grain::Day),
                vec!["sunday", "sun", "sun."],
            ),
            examples(
                datetime(2013, 3, 1, 0, 0, 0, Grain::Day),
                vec![
                    "the 1st of march",
                    "first of march",
                    "the first of march",
                    "march first",
                ],
            ),
            examples(
                datetime(2013, 3, 2, 0, 0, 0, Grain::Day),
                vec!["the 2nd of march", "second of march", "the second of march"],
            ),
            examples(
                datetime(2013, 3, 3, 0, 0, 0, Grain::Day),
                vec!["march 3", "the third of march"],
            ),
            examples(
                datetime(2013, 3, 15, 0, 0, 0, Grain::Day),
                vec!["the ides of march"],
            ),
            examples(
                datetime(2015, 3, 3, 0, 0, 0, Grain::Day),
                vec![
                    "march 3 2015",
                    "march 3rd 2015",
                    "march third 2015",
                    "3/3/2015",
                    "3/3/15",
                    "2015-3-3",
                    "2015-03-03",
                ],
            ),
            examples(
                datetime(2013, 2, 15, 0, 0, 0, Grain::Day),
                vec![
                    "on the 15th",
                    "the 15th of february",
                    "15 of february",
                    "february the 15th",
                    "february 15",
                    "15th february",
                    "February 15",
                ],
            ),
            examples(datetime(2013, 8, 8, 0, 0, 0, Grain::Day), vec!["Aug 8"]),
            examples(
                datetime(2014, 3, 1, 0, 0, 0, Grain::Month),
                vec!["March in 1 year", "March in a year"],
            ),
            examples(
                datetime(2014, 7, 18, 0, 0, 0, Grain::Day),
                vec!["Fri, Jul 18", "Jul 18, Fri"],
            ),
            examples(
                datetime(2014, 10, 1, 0, 0, 0, Grain::Month),
                vec!["October 2014", "2014-10", "2014/10"],
            ),
            examples(
                datetime(2015, 4, 14, 0, 0, 0, Grain::Day),
                vec!["14april 2015", "April 14, 2015", "14th April 15"],
            ),
            examples(
                datetime(2013, 2, 19, 0, 0, 0, Grain::Day),
                vec!["next tuesday", "around next tuesday"],
            ),
            examples(
                datetime(2013, 2, 22, 0, 0, 0, Grain::Day),
                vec!["friday after next"],
            ),
            examples(
                datetime(2013, 3, 1, 0, 0, 0, Grain::Month),
                vec!["next March"],
            ),
            examples(
                datetime(2014, 3, 1, 0, 0, 0, Grain::Month),
                vec!["March after next"],
            ),
            examples(
                datetime(2013, 2, 10, 0, 0, 0, Grain::Day),
                vec!["Sunday, Feb 10"],
            ),
            examples(
                datetime(2013, 2, 13, 0, 0, 0, Grain::Day),
                vec!["Wed, Feb13"],
            ),
            examples(
                datetime(2013, 2, 11, 0, 0, 0, Grain::Week),
                vec!["this week", "current week"],
            ),
            examples(
                datetime(2013, 2, 4, 0, 0, 0, Grain::Week),
                vec!["last week", "past week", "previous week"],
            ),
            examples(
                datetime(2013, 2, 18, 0, 0, 0, Grain::Week),
                vec![
                    "next week",
                    "the following week",
                    "around next week",
                    "upcoming week",
                    "coming week",
                ],
            ),
            examples(
                datetime(2013, 1, 1, 0, 0, 0, Grain::Month),
                vec!["last month"],
            ),
            examples(
                datetime(2013, 3, 1, 0, 0, 0, Grain::Month),
                vec!["next month"],
            ),
            examples(
                datetime(2013, 3, 20, 0, 0, 0, Grain::Day),
                vec![
                    "20 of next month",
                    "20th of the next month",
                    "20th day of next month",
                ],
            ),
            examples(
                datetime(2013, 2, 20, 0, 0, 0, Grain::Day),
                vec!["20th of the current month", "20 of this month"],
            ),
            examples(
                datetime(2013, 1, 20, 0, 0, 0, Grain::Day),
                vec!["20th of the previous month"],
            ),
            examples(
                datetime(2013, 1, 1, 0, 0, 0, Grain::Quarter),
                vec!["this quarter", "this qtr"],
            ),
            examples(
                datetime(2013, 4, 1, 0, 0, 0, Grain::Quarter),
                vec!["next quarter", "next qtr"],
            ),
            examples(
                datetime(2013, 7, 1, 0, 0, 0, Grain::Quarter),
                vec![
                    "third quarter",
                    "3rd quarter",
                    "third qtr",
                    "3rd qtr",
                    "the 3rd qtr",
                ],
            ),
            examples(
                datetime(2018, 10, 1, 0, 0, 0, Grain::Quarter),
                vec![
                    "4th quarter 2018",
                    "4th qtr 2018",
                    "the 4th qtr of 2018",
                    "18q4",
                    "2018Q4",
                ],
            ),
            examples(
                datetime(2012, 1, 1, 0, 0, 0, Grain::Year),
                vec!["last year", "last yr"],
            ),
            examples(
                datetime(2013, 1, 1, 0, 0, 0, Grain::Year),
                vec!["this year", "current year", "this yr"],
            ),
            examples(
                datetime(2014, 1, 1, 0, 0, 0, Grain::Year),
                vec!["next year", "next yr"],
            ),
            examples(
                datetime(2014, 1, 1, 0, 0, 0, Grain::Year),
                vec!["in 2014 A.D.", "in 2014 AD"],
            ),
            examples(
                datetime(-2014, 1, 1, 0, 0, 0, Grain::Year),
                vec!["in 2014 B.C.", "in 2014 BC"],
            ),
            examples(datetime(14, 1, 1, 0, 0, 0, Grain::Year), vec!["in 14 a.d."]),
            examples(
                datetime(2013, 2, 10, 0, 0, 0, Grain::Day),
                vec!["last sunday", "sunday from last week", "last week's sunday"],
            ),
            examples(
                datetime(2013, 2, 5, 0, 0, 0, Grain::Day),
                vec!["last tuesday"],
            ),
            examples(
                datetime(2013, 2, 20, 0, 0, 0, Grain::Day),
                vec!["next wednesday"],
            ),
            examples(
                datetime(2013, 2, 20, 0, 0, 0, Grain::Day),
                vec![
                    "wednesday of next week",
                    "wednesday next week",
                    "wednesday after next",
                ],
            ),
            examples(
                datetime(2013, 2, 22, 0, 0, 0, Grain::Day),
                vec!["friday after next"],
            ),
            examples(
                datetime(2013, 2, 11, 0, 0, 0, Grain::Day),
                vec!["monday of this week"],
            ),
            examples(
                datetime(2013, 2, 12, 0, 0, 0, Grain::Day),
                vec!["tuesday of this week"],
            ),
            examples(
                datetime(2013, 2, 13, 0, 0, 0, Grain::Day),
                vec!["wednesday of this week"],
            ),
            examples(
                datetime(2013, 2, 14, 0, 0, 0, Grain::Day),
                vec!["the day after tomorrow"],
            ),
            examples(
                datetime(2013, 2, 14, 17, 0, 0, Grain::Hour),
                vec!["day after tomorrow 5pm"],
            ),
            examples(
                datetime(2013, 2, 10, 0, 0, 0, Grain::Day),
                vec!["the day before yesterday"],
            ),
            examples(
                datetime(2013, 2, 10, 8, 0, 0, Grain::Hour),
                vec!["day before yesterday 8am"],
            ),
            examples(
                datetime(2013, 3, 25, 0, 0, 0, Grain::Day),
                vec!["last Monday of March"],
            ),
            examples(
                datetime(2014, 3, 30, 0, 0, 0, Grain::Day),
                vec!["last Sunday of March 2014"],
            ),
            examples(
                datetime(2013, 10, 3, 0, 0, 0, Grain::Day),
                vec!["third day of october"],
            ),
            examples(
                datetime(2014, 10, 6, 0, 0, 0, Grain::Week),
                vec!["first week of october 2014"],
            ),
            examples(
                datetime(2018, 12, 10, 0, 0, 0, Grain::Week),
                vec![
                    "third last week of 2018",
                    "the third last week of 2018",
                    "the 3rd last week of 2018",
                ],
            ),
            examples(
                datetime(2018, 10, 15, 0, 0, 0, Grain::Week),
                vec![
                    "2nd last week of October 2018",
                    "the second last week of October 2018",
                ],
            ),
            examples(
                datetime(2013, 5, 27, 0, 0, 0, Grain::Day),
                vec!["fifth last day of May", "the 5th last day of May"],
            ),
            examples(
                datetime(2013, 10, 7, 0, 0, 0, Grain::Week),
                vec!["the week of october 6th"],
            ),
            examples(
                datetime(2013, 10, 7, 0, 0, 0, Grain::Week),
                vec!["the week of october 7th"],
            ),
            examples(
                datetime(2015, 10, 31, 0, 0, 0, Grain::Day),
                vec!["last day of october 2015", "last day in october 2015"],
            ),
            examples(
                datetime(2014, 9, 22, 0, 0, 0, Grain::Week),
                vec!["last week of september 2014"],
            ),
            examples(
                datetime(2013, 10, 1, 0, 0, 0, Grain::Day),
                vec!["first tuesday of october", "first tuesday in october"],
            ),
            examples(
                datetime(2014, 9, 16, 0, 0, 0, Grain::Day),
                vec!["third tuesday of september 2014"],
            ),
            examples(
                datetime(2014, 10, 1, 0, 0, 0, Grain::Day),
                vec!["first wednesday of october 2014"],
            ),
            examples(
                datetime(2014, 10, 8, 0, 0, 0, Grain::Day),
                vec!["second wednesday of october 2014"],
            ),
            examples(
                datetime(2015, 1, 13, 0, 0, 0, Grain::Day),
                vec!["third tuesday after christmas 2014"],
            ),
            examples(
                datetime(2013, 2, 13, 3, 0, 0, Grain::Hour),
                vec![
                    "at 3am",
                    "3 in the AM",
                    "at 3 AM",
                    "3 oclock am",
                    "at three am",
                    "this morning at 3",
                    "3 in the morning",
                    "at 3 in the morning",
                    "early morning @ 3",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 10, 0, 0, Grain::Hour),
                vec!["this morning @ 10", "this morning at 10am"],
            ),
            examples(
                datetime(2013, 2, 13, 3, 18, 0, Grain::Minute),
                vec!["3:18am", "3:18a", "3h18"],
            ),
            examples(
                datetime(2016, 2, 1, 7, 0, 0, Grain::Hour),
                vec!["at 7 in 3 years"],
            ),
            examples(
                datetime(2013, 2, 12, 15, 0, 0, Grain::Hour),
                vec![
                    "at 3pm",
                    "@ 3pm",
                    "3PM",
                    "3pm",
                    "3 oclock pm",
                    "3 o'clock in the afternoon",
                    "3ish pm",
                    "3pm approximately",
                    "at about 3pm",
                    "at 3p",
                    "at 3p.",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 15, 0, 0, Grain::Minute),
                vec!["15h00", "at 15h00", "15h", "at 15h"],
            ),
            examples(
                datetime(2013, 2, 12, 15, 15, 0, Grain::Minute),
                vec![
                    "at 15 past 3pm",
                    "a quarter past 3pm",
                    "for a quarter past 3pm",
                    "3:15 in the afternoon",
                    "15:15",
                    "15h15",
                    "3:15pm",
                    "3:15PM",
                    "3:15p",
                    "at 3 15",
                    "15 minutes past 3pm",
                    "15 minutes past 15h",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 15, 20, 0, Grain::Minute),
                vec![
                    "at 20 past 3pm",
                    "3:20 in the afternoon",
                    "3:20 in afternoon",
                    "twenty after 3pm",
                    "3:20p",
                    "15h20",
                    "at three twenty",
                    "20 minutes past 3pm",
                    "this afternoon at 3:20",
                    "tonight @ 3:20",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 15, 30, 0, Grain::Minute),
                vec![
                    "at half past three pm",
                    "half past 3 pm",
                    "15:30",
                    "15h30",
                    "3:30pm",
                    "3:30PM",
                    "330 p.m.",
                    "3:30 p m",
                    "3:30",
                    "half three",
                    "30 minutes past 3 pm",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 12, 15, 0, Grain::Minute),
                vec![
                    "at 15 past noon",
                    "a quarter past noon",
                    "for a quarter past noon",
                    "12:15 in the afternoon",
                    "12:15",
                    "12h15",
                    "12:15pm",
                    "12:15PM",
                    "12:15p",
                    "at 12 15",
                    "15 minutes past noon",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 9, 59, 0, Grain::Minute),
                vec!["nine fifty nine a m"],
            ),
            examples(
                datetime(2013, 2, 12, 15, 23, 24, Grain::Second),
                vec!["15:23:24"],
            ),
            examples(
                datetime(2013, 2, 12, 9, 1, 10, Grain::Second),
                vec!["9:01:10 AM"],
            ),
            examples(
                datetime(2013, 2, 12, 11, 45, 0, Grain::Minute),
                vec!["a quarter to noon", "11:45am", "11h45", "15 to noon"],
            ),
            examples(
                datetime(2013, 2, 12, 13, 15, 0, Grain::Minute),
                vec![
                    "a quarter past 1pm",
                    "for a quarter past 1pm",
                    "1:15pm",
                    "13h15",
                    "15 minutes from 1pm",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 14, 15, 0, Grain::Minute),
                vec!["a quarter past 2pm", "for a quarter past 2pm"],
            ),
            examples(
                datetime(2013, 2, 12, 20, 15, 0, Grain::Minute),
                vec!["a quarter past 8pm", "for a quarter past 8pm"],
            ),
            examples(
                datetime(2013, 2, 12, 20, 0, 0, Grain::Hour),
                vec![
                    "8 tonight",
                    "tonight at 8 o'clock",
                    "eight tonight",
                    "8 this evening",
                    "at 8 in the evening",
                    "in the evening at eight",
                ],
            ),
            examples(
                datetime(2013, 9, 20, 19, 30, 0, Grain::Minute),
                vec!["at 7:30 PM on Fri, Sep 20", "at 19h30 on Fri, Sep 20"],
            ),
            examples(
                datetime(2013, 2, 16, 9, 0, 0, Grain::Hour),
                vec!["at 9am on Saturday", "Saturday morning at 9"],
            ),
            examples(
                datetime(2013, 2, 16, 9, 0, 0, Grain::Hour),
                vec!["on Saturday for 9am"],
            ),
            examples(
                datetime(2014, 7, 18, 19, 0, 0, Grain::Minute),
                vec![
                    "Fri, Jul 18, 2014 07:00 PM",
                    "Fri, Jul 18, 2014 19h00",
                    "Fri, Jul 18, 2014 19h",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 4, 30, 1, Grain::Second),
                vec!["in a sec", "one second from now", "in 1\""],
            ),
            examples(
                datetime(2013, 2, 12, 4, 31, 0, Grain::Second),
                vec!["in a minute", "in one minute", "in 1'"],
            ),
            examples(
                datetime(2013, 2, 12, 4, 32, 0, Grain::Second),
                vec![
                    "in 2 minutes",
                    "in 2 more minutes",
                    "2 minutes from now",
                    "in a couple of minutes",
                    "in a pair of minutes",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 4, 33, 0, Grain::Second),
                vec!["in three minutes", "in a few minutes"],
            ),
            examples(
                datetime(2013, 2, 12, 5, 30, 0, Grain::Second),
                vec!["in 60 minutes"],
            ),
            examples(
                datetime(2013, 2, 12, 4, 45, 0, Grain::Second),
                vec![
                    "in a quarter of an hour",
                    "in 1/4h",
                    "in 1/4 h",
                    "in 1/4 hour",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 5, 0, 0, Grain::Second),
                vec!["in half an hour", "in 1/2h", "in 1/2 h", "in 1/2 hour"],
            ),
            examples(
                datetime(2013, 2, 12, 5, 15, 0, Grain::Second),
                vec![
                    "in three-quarters of an hour",
                    "in 3/4h",
                    "in 3/4 h",
                    "in 3/4 hour",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 7, 0, 0, Grain::Second),
                vec!["in 2.5 hours", "in 2 and an half hours"],
            ),
            examples(
                datetime(2013, 2, 12, 5, 30, 0, Grain::Minute),
                vec!["in one hour", "in 1h"],
            ),
            examples(
                datetime(2013, 2, 12, 6, 30, 0, Grain::Minute),
                vec!["in a couple hours", "in a couple of hours"],
            ),
            examples(
                datetime(2013, 2, 12, 7, 30, 0, Grain::Minute),
                vec!["in a few hours", "in few hours"],
            ),
            examples(
                datetime(2013, 2, 13, 4, 30, 0, Grain::Minute),
                vec!["in 24 hours"],
            ),
            examples(
                datetime(2013, 2, 13, 4, 0, 0, Grain::Hour),
                vec!["in a day", "a day from now"],
            ),
            examples(
                datetime(2013, 2, 13, 4, 30, 0, Grain::Second),
                vec!["a day from right now"],
            ),
            examples(
                datetime(2016, 2, 12, 0, 0, 0, Grain::Day),
                vec!["3 years from today"],
            ),
            examples(
                datetime(2013, 3, 1, 0, 0, 0, Grain::Day),
                vec!["3 fridays from now", "three fridays from now"],
            ),
            examples(
                datetime(2013, 2, 24, 0, 0, 0, Grain::Day),
                vec!["2 sundays from now", "two sundays from now"],
            ),
            examples(
                datetime(2013, 3, 12, 0, 0, 0, Grain::Day),
                vec!["4 tuesdays from now", "four tuesdays from now"],
            ),
            examples(
                datetime(2013, 2, 19, 4, 0, 0, Grain::Hour),
                vec!["in 7 days"],
            ),
            examples(
                datetime(2013, 2, 19, 17, 0, 0, Grain::Hour),
                vec!["in 7 days at 5pm"],
            ),
            examples(
                datetime(2017, 2, 1, 17, 0, 0, Grain::Hour),
                vec!["in 4 years at 5pm"],
            ),
            examples(
                datetime(2013, 2, 19, 0, 0, 0, Grain::Day),
                vec!["in 1 week", "in a week"],
            ),
            examples(
                datetime(2013, 2, 12, 5, 0, 0, Grain::Second),
                vec!["in about half an hour"],
            ),
            examples(
                datetime(2013, 2, 5, 4, 0, 0, Grain::Hour),
                vec!["7 days ago"],
            ),
            examples(
                datetime(2013, 1, 29, 4, 0, 0, Grain::Hour),
                vec!["14 days Ago", "a fortnight ago"],
            ),
            examples(
                datetime(2013, 2, 5, 0, 0, 0, Grain::Day),
                vec!["a week ago", "one week ago", "1 week ago"],
            ),
            examples(
                datetime(2013, 1, 31, 0, 0, 0, Grain::Day),
                vec!["2 thursdays back", "2 thursdays ago"],
            ),
            examples(
                datetime(2013, 1, 22, 0, 0, 0, Grain::Day),
                vec!["three weeks ago"],
            ),
            examples(
                datetime(2012, 11, 12, 0, 0, 0, Grain::Day),
                vec!["three months ago"],
            ),
            examples(
                datetime(2013, 2, 4, 0, 0, 0, Grain::Day),
                vec![
                    "the first Monday of this month",
                    "the first Monday of the month",
                    "the first Monday in this month",
                    "first Monday in the month",
                ],
            ),
            examples(
                datetime(2011, 2, 1, 0, 0, 0, Grain::Month),
                vec!["two years ago"],
            ),
            examples(
                datetime(2013, 2, 19, 4, 0, 0, Grain::Hour),
                vec!["7 days hence"],
            ),
            examples(
                datetime(2013, 2, 26, 4, 0, 0, Grain::Hour),
                vec!["14 days hence", "a fortnight hence"],
            ),
            examples(
                datetime(2013, 2, 19, 0, 0, 0, Grain::Day),
                vec!["a week hence", "one week hence", "1 week hence"],
            ),
            examples(
                datetime(2013, 3, 5, 0, 0, 0, Grain::Day),
                vec!["three weeks hence"],
            ),
            examples(
                datetime(2013, 5, 12, 0, 0, 0, Grain::Day),
                vec!["three months hence"],
            ),
            examples(
                datetime(2015, 2, 1, 0, 0, 0, Grain::Month),
                vec!["two years hence"],
            ),
            examples(
                datetime(2013, 12, 25, 0, 0, 0, Grain::Day),
                vec!["one year After christmas", "a year from Christmas"],
            ),
            examples(
                datetime_interval(2013, 12, 18, 0, 0, 0, 2013, 12, 29, 0, 0, 0, Grain::Day),
                vec![
                    "for 10 days from 18th Dec",
                    "from 18th Dec for 10 days",
                    "18th Dec for 10 days",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 12, 16, 0, 0, 2013, 2, 12, 16, 31, 0, Grain::Minute),
                vec![
                    "for 30' starting from 4pm",
                    "from 4pm for thirty minutes",
                    "4pm for 30 mins",
                    "16h for 30 mins",
                ],
            ),
            examples(
                datetime_interval(2013, 6, 21, 0, 0, 0, 2013, 9, 24, 0, 0, 0, Grain::Day),
                vec!["this Summer", "current summer"],
            ),
            examples(
                datetime_interval(2012, 12, 21, 0, 0, 0, 2013, 3, 21, 0, 0, 0, Grain::Day),
                vec!["this winter"],
            ),
            examples(
                datetime_interval(2012, 12, 21, 0, 0, 0, 2013, 3, 19, 0, 0, 0, Grain::Day),
                vec!["this season", "current seasons"],
            ),
            examples(
                datetime_interval(2012, 9, 23, 0, 0, 0, 2012, 12, 20, 0, 0, 0, Grain::Day),
                vec!["last season", "past seasons", "previous seasons"],
            ),
            examples(
                datetime_interval(2013, 3, 20, 0, 0, 0, 2013, 6, 20, 0, 0, 0, Grain::Day),
                vec!["next season"],
            ),
            examples(
                datetime_interval(2013, 2, 11, 18, 0, 0, 2013, 2, 12, 0, 0, 0, Grain::Hour),
                vec!["last night", "yesterday evening"],
            ),
            examples(
                datetime_interval(2013, 2, 11, 21, 0, 0, 2013, 2, 12, 0, 0, 0, Grain::Hour),
                vec!["late last night"],
            ),
            examples(
                datetime_holiday(2013, 12, 25, 0, 0, 0, Grain::Day, "Christmas"),
                vec!["xmas", "christmas", "christmas day"],
            ),
            examples(
                datetime_holiday(2013, 12, 25, 18, 0, 0, Grain::Hour, "Christmas"),
                vec!["xmas at 6 pm"],
            ),
            examples(
                datetime_interval_holiday(
                    2013,
                    12,
                    25,
                    0,
                    0,
                    0,
                    2013,
                    12,
                    25,
                    12,
                    0,
                    0,
                    Grain::Hour,
                    "Christmas",
                ),
                vec![
                    "morning of xmas",
                    "morning of christmas 2013",
                    "morning of this christmas day",
                ],
            ),
            examples(
                datetime_holiday(2013, 12, 31, 0, 0, 0, Grain::Day, "New Year's Eve"),
                vec!["new year's eve", "new years eve"],
            ),
            examples(
                datetime_holiday(2014, 1, 1, 0, 0, 0, Grain::Day, "New Year's Day"),
                vec!["new year's day", "new years day"],
            ),
            examples(
                datetime_holiday(2013, 2, 14, 0, 0, 0, Grain::Day, "Valentine's Day"),
                vec!["valentine's day", "valentine day"],
            ),
            examples(
                datetime(2013, 7, 4, 0, 0, 0, Grain::Day),
                vec!["4th of July", "4 of july"],
            ),
            examples(
                datetime_holiday(2013, 10, 31, 0, 0, 0, Grain::Day, "Halloween"),
                vec!["halloween", "next halloween", "Halloween 2013"],
            ),
            examples(
                datetime_holiday(2013, 11, 29, 0, 0, 0, Grain::Day, "Black Friday"),
                vec![
                    "black friday",
                    "black friday of this year",
                    "black friday 2013",
                ],
            ),
            examples(
                datetime_holiday(2017, 11, 24, 0, 0, 0, Grain::Day, "Black Friday"),
                vec!["black friday 2017"],
            ),
            examples(
                datetime_holiday(2013, 10, 16, 0, 0, 0, Grain::Day, "Boss's Day"),
                vec!["boss's day", "boss's", "boss day", "next boss's day"],
            ),
            examples(
                datetime_holiday(2016, 10, 17, 0, 0, 0, Grain::Day, "Boss's Day"),
                vec!["boss's day 2016"],
            ),
            examples(
                datetime_holiday(2021, 10, 15, 0, 0, 0, Grain::Day, "Boss's Day"),
                vec!["boss's day 2021"],
            ),
            examples(
                datetime_holiday(2014, 1, 20, 0, 0, 0, Grain::Day, "Martin Luther King's Day"),
                vec![
                    "MLK day",
                    "next Martin Luther King day",
                    "next Martin Luther King's day",
                    "next Martin Luther Kings day",
                    "this MLK day",
                ],
            ),
            examples(
                datetime_holiday(2013, 1, 21, 0, 0, 0, Grain::Day, "Martin Luther King's Day"),
                vec!["last MLK Jr. day", "MLK day 2013"],
            ),
            examples(
                datetime_holiday(2012, 1, 16, 0, 0, 0, Grain::Day, "Martin Luther King's Day"),
                vec![
                    "MLK day of last year",
                    "MLK day 2012",
                    "Civil Rights Day of last year",
                ],
            ),
            examples(
                datetime_holiday(2013, 11, 1, 0, 0, 0, Grain::Day, "World Vegan Day"),
                vec!["world vegan day"],
            ),
            examples(
                datetime_holiday(2013, 3, 31, 0, 0, 0, Grain::Day, "Easter Sunday"),
                vec!["easter", "easter 2013"],
            ),
            examples(
                datetime_holiday(2012, 4, 8, 0, 0, 0, Grain::Day, "Easter Sunday"),
                vec!["last easter"],
            ),
            examples(
                datetime_holiday(2013, 4, 1, 0, 0, 0, Grain::Day, "Easter Monday"),
                vec!["easter mon"],
            ),
            examples(
                datetime_holiday(2010, 4, 4, 0, 0, 0, Grain::Day, "Easter Sunday"),
                vec!["easter 2010", "Easter Sunday two thousand ten"],
            ),
            examples(
                datetime(2013, 4, 3, 0, 0, 0, Grain::Day),
                vec!["three days after Easter"],
            ),
            examples(
                datetime_holiday(2013, 3, 28, 0, 0, 0, Grain::Day, "Maundy Thursday"),
                vec!["Maundy Thursday", "Covenant thu", "Thu of Mysteries"],
            ),
            examples(
                datetime_holiday(2013, 5, 19, 0, 0, 0, Grain::Day, "Pentecost"),
                vec!["Pentecost", "white sunday 2013"],
            ),
            examples(
                datetime_holiday(2013, 5, 20, 0, 0, 0, Grain::Day, "Whit Monday"),
                vec!["whit monday", "Monday of the Holy Spirit"],
            ),
            examples(
                datetime_holiday(2013, 3, 24, 0, 0, 0, Grain::Day, "Palm Sunday"),
                vec!["palm sunday", "branch sunday 2013"],
            ),
            examples(
                datetime_holiday(2013, 5, 26, 0, 0, 0, Grain::Day, "Trinity Sunday"),
                vec!["trinity sunday"],
            ),
            examples(
                datetime_holiday(2013, 2, 12, 0, 0, 0, Grain::Day, "Shrove Tuesday"),
                vec!["pancake day 2013", "mardi gras"],
            ),
            examples(
                datetime_holiday(2013, 3, 17, 0, 0, 0, Grain::Day, "St Patrick's Day"),
                vec![
                    "st patrick's day 2013",
                    "st paddy's day",
                    "saint paddy's day",
                    "saint patricks day",
                ],
            ),
            examples(
                datetime_interval_holiday(
                    2018,
                    2,
                    14,
                    0,
                    0,
                    0,
                    2018,
                    4,
                    1,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Lent",
                ),
                vec!["lent 2018"],
            ),
            examples(
                datetime_holiday(2018, 4, 8, 0, 0, 0, Grain::Day, "Orthodox Easter Sunday"),
                vec!["orthodox easter 2018"],
            ),
            examples(
                datetime_holiday(2020, 4, 17, 0, 0, 0, Grain::Day, "Orthodox Good Friday"),
                vec!["orthodox good friday 2020", "orthodox great friday 2020"],
            ),
            examples(
                datetime_holiday(2018, 2, 19, 0, 0, 0, Grain::Day, "Clean Monday"),
                vec![
                    "clean monday 2018",
                    "orthodox shrove monday two thousand eighteen",
                ],
            ),
            examples(
                datetime_holiday(2018, 3, 31, 0, 0, 0, Grain::Day, "Lazarus Saturday"),
                vec!["lazarus saturday 2018"],
            ),
            examples(
                datetime_interval_holiday(
                    2018,
                    2,
                    19,
                    0,
                    0,
                    0,
                    2018,
                    3,
                    31,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Great Lent",
                ),
                vec!["great fast 2018"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 18, 0, 0, 2013, 2, 13, 0, 0, 0, Grain::Hour),
                vec!["this evening", "today evening", "tonight"],
            ),
            examples(
                datetime_interval(2013, 2, 8, 18, 0, 0, 2013, 2, 11, 0, 0, 0, Grain::Hour),
                vec!["this past weekend"],
            ),
            examples(
                datetime_interval(2013, 2, 13, 18, 0, 0, 2013, 2, 14, 0, 0, 0, Grain::Hour),
                vec!["tomorrow evening"],
            ),
            examples(
                datetime_interval(2013, 2, 13, 12, 0, 0, 2013, 2, 13, 14, 0, 0, Grain::Hour),
                vec!["tomorrow lunch", "tomorrow at lunch"],
            ),
            examples(
                datetime_interval(2013, 2, 11, 18, 0, 0, 2013, 2, 12, 0, 0, 0, Grain::Hour),
                vec!["yesterday evening"],
            ),
            examples(
                datetime_interval(2013, 2, 15, 18, 0, 0, 2013, 2, 18, 0, 0, 0, Grain::Hour),
                vec!["this week-end"],
            ),
            examples(
                datetime_interval(2013, 2, 18, 0, 0, 0, 2013, 2, 18, 12, 0, 0, Grain::Hour),
                vec!["monday mOrnIng"],
            ),
            examples(
                datetime_interval(2013, 2, 18, 0, 0, 0, 2013, 2, 18, 9, 0, 0, Grain::Hour),
                vec![
                    "monday early in the morning",
                    "monday early morning",
                    "monday in the early hours of the morning",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 12, 21, 0, 0, 2013, 2, 13, 0, 0, 0, Grain::Hour),
                vec!["late tonight", "late tonite"],
            ),
            examples(
                datetime_interval(2013, 2, 15, 0, 0, 0, 2013, 2, 15, 12, 0, 0, Grain::Hour),
                vec![
                    "february the 15th in the morning",
                    "15 of february in the morning",
                    "morning of the 15th of february",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 29, 58, 2013, 2, 12, 4, 30, 0, Grain::Second),
                vec!["last 2 seconds", "last two seconds"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 30, 1, 2013, 2, 12, 4, 30, 4, Grain::Second),
                vec!["next 3 seconds", "next three seconds"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 28, 0, 2013, 2, 12, 4, 30, 0, Grain::Minute),
                vec!["last 2 minutes", "last two minutes"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 31, 0, 2013, 2, 12, 4, 34, 0, Grain::Minute),
                vec!["next 3 minutes", "next three minutes"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 3, 0, 0, 2013, 2, 12, 4, 0, 0, Grain::Hour),
                vec!["last 1 hour", "last one hour"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 5, 0, 0, 2013, 2, 12, 8, 0, 0, Grain::Hour),
                vec!["next 3 hours", "next three hours"],
            ),
            examples(
                datetime_interval(2013, 2, 10, 0, 0, 0, 2013, 2, 12, 0, 0, 0, Grain::Day),
                vec!["last 2 days", "last two days", "past 2 days"],
            ),
            examples(
                datetime_interval(2013, 2, 13, 0, 0, 0, 2013, 2, 16, 0, 0, 0, Grain::Day),
                vec!["next 3 days", "next three days"],
            ),
            examples(
                datetime_interval(2013, 2, 13, 0, 0, 0, 2013, 2, 16, 0, 0, 0, Grain::Day),
                vec!["next few days"],
            ),
            examples(
                datetime_interval(2013, 1, 28, 0, 0, 0, 2013, 2, 11, 0, 0, 0, Grain::Week),
                vec!["last 2 weeks", "last two weeks", "past 2 weeks"],
            ),
            examples(
                datetime_interval(2013, 2, 18, 0, 0, 0, 2013, 3, 11, 0, 0, 0, Grain::Week),
                vec!["next 3 weeks", "next three weeks"],
            ),
            examples(
                datetime_interval(2012, 12, 1, 0, 0, 0, 2013, 2, 1, 0, 0, 0, Grain::Month),
                vec!["last 2 months", "last two months"],
            ),
            examples(
                datetime_interval(2013, 3, 1, 0, 0, 0, 2013, 6, 1, 0, 0, 0, Grain::Month),
                vec!["next 3 months", "next three months"],
            ),
            examples(
                datetime_interval(2011, 1, 1, 0, 0, 0, 2013, 1, 1, 0, 0, 0, Grain::Year),
                vec!["last 2 years", "last two years"],
            ),
            examples(
                datetime_interval(2014, 1, 1, 0, 0, 0, 2017, 1, 1, 0, 0, 0, Grain::Year),
                vec!["next 3 years", "next three years"],
            ),
            examples(
                datetime_interval(2013, 7, 13, 0, 0, 0, 2013, 7, 16, 0, 0, 0, Grain::Day),
                vec![
                    "July 13-15",
                    "July 13 to 15",
                    "July 13 thru 15",
                    "July 13 through 15",
                    "July 13 - July 15",
                ],
            ),
            examples(
                datetime_interval(2013, 7, 13, 0, 0, 0, 2013, 7, 16, 0, 0, 0, Grain::Day),
                vec![
                    "from July 13-15",
                    "from 13 to 15 July",
                    "from 13th to 15th July",
                    "from the 13 to 15 July",
                    "from the 13th to 15th July",
                    "from the 13th to the 15th July",
                    "from the 13 to the 15 July",
                ],
            ),
            examples(
                datetime_interval(2013, 7, 13, 0, 0, 0, 2013, 7, 16, 0, 0, 0, Grain::Day),
                vec![
                    "from 13 to 15 of July",
                    "from 13th to 15 of July",
                    "from 13 to 15th of July",
                    "from 13th to 15th of July",
                    "from 13 to the 15 of July",
                    "from 13th to the 15 of July",
                    "from 13 to the 15th of July",
                    "from 13th to the 15th of July",
                    "from the 13 to the 15 of July",
                    "from the 13th to the 15 of July",
                    "from the 13 to the 15th of July",
                    "from the 13th to the 15th of July",
                ],
            ),
            examples(
                datetime_interval(2013, 8, 8, 0, 0, 0, 2013, 8, 13, 0, 0, 0, Grain::Day),
                vec!["Aug 8 - Aug 12"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 9, 30, 0, 2013, 2, 12, 11, 1, 0, Grain::Minute),
                vec!["9:30 - 11:00", "9h30 - 11h00", "9h30 - 11h"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 13, 30, 0, 2013, 2, 12, 15, 1, 0, Grain::Minute),
                vec!["9:30 - 11:00 CST", "9h30 - 11h00 CST", "9h30 - 11h CST"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 13, 0, 0, 2013, 2, 12, 16, 1, 0, Grain::Minute),
                vec![
                    "15:00 GMT - 18:00 GMT",
                    "15h00 GMT - 18h00 GMT",
                    "15h GMT - 18h GMT",
                ],
            ),
            examples(
                datetime_interval(2015, 3, 28, 17, 0, 0, 2015, 3, 29, 21, 0, 1, Grain::Second),
                vec!["2015-03-28 17:00:00/2015-03-29 21:00:00"],
            ),
            examples(
                datetime_interval(2013, 2, 14, 9, 30, 0, 2013, 2, 14, 11, 1, 0, Grain::Minute),
                vec![
                    "from 9:30 - 11:00 on Thursday",
                    "between 9:30 and 11:00 on thursday",
                    "between 9h30 and 11h00 on thursday",
                    "9:30 - 11:00 on Thursday",
                    "9h30 - 11h00 on Thursday",
                    "later than 9:30 but before 11:00 on Thursday",
                    "Thursday from 9:30 to 11:00",
                    "from 9:30 untill 11:00 on thursday",
                    "Thursday from 9:30 untill 11:00",
                    "9:30 till 11:00 on Thursday",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 13, 1, 0, 0, 2013, 2, 13, 2, 31, 0, Grain::Minute),
                vec!["tomorrow in between 1-2:30 ish"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 15, 0, 0, 2013, 2, 12, 17, 0, 0, Grain::Hour),
                vec!["3-4pm", "from 3 to 4 in the PM", "around 3-4pm"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 15, 30, 0, 2013, 2, 12, 18, 1, 0, Grain::Minute),
                vec![
                    "3:30 to 6 PM",
                    "3:30-6 p.m.",
                    "3:30-6:00pm",
                    "15h30-18h",
                    "from 3:30 to six p.m.",
                    "from 3:30 to 6:00pm",
                    "later than 3:30pm but before 6pm",
                    "between 3:30pm and 6 pm",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 12, 15, 0, 0, 2013, 2, 12, 18, 0, 1, Grain::Second),
                vec!["3pm - 6:00:00pm"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 8, 0, 0, 2013, 2, 12, 14, 0, 0, Grain::Hour),
                vec!["8am - 1pm"],
            ),
            examples(
                datetime_interval(2013, 2, 14, 9, 0, 0, 2013, 2, 14, 12, 0, 0, Grain::Hour),
                vec!["Thursday from 9a to 11a", "this Thu 9-11am"],
            ),
            examples(
                datetime_interval(
                    2013,
                    2,
                    12,
                    11,
                    30,
                    0,
                    2013,
                    2,
                    12,
                    13,
                    31,
                    0,
                    Grain::Minute,
                ),
                vec!["11:30-1:30"],
            ),
            examples(
                datetime(2013, 9, 21, 13, 30, 0, Grain::Minute),
                vec!["1:30 PM on Sat, Sep 21"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 30, 0, 2013, 2, 26, 0, 0, 0, Grain::Second),
                vec!["Within 2 weeks"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 30, 0, 2013, 2, 12, 14, 0, 0, Grain::Second),
                vec!["by 2:00pm"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 30, 0, 2013, 2, 13, 0, 0, 0, Grain::Second),
                vec!["by EOD"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 30, 0, 2013, 3, 1, 0, 0, 0, Grain::Second),
                vec![
                    "by EOM",
                    "by the EOM",
                    "by end of the month",
                    "by the end of month",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 21, 0, 0, 0, 2013, 3, 1, 0, 0, 0, Grain::Day),
                vec![
                    "EOM",
                    "the EOM",
                    "at the EOM",
                    "the end of the month",
                    "end of the month",
                    "at the end of month",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 1, 0, 0, 0, 2013, 2, 11, 0, 0, 0, Grain::Day),
                vec![
                    "BOM",
                    "the BOM",
                    "at the BOM",
                    "beginning of the month",
                    "the beginning of the month",
                    "at the beginning of month",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 30, 0, 2013, 4, 1, 0, 0, 0, Grain::Second),
                vec!["by the end of next month"],
            ),
            examples(
                datetime(2013, 2, 12, 13, 0, 0, Grain::Minute),
                vec!["4pm CET"],
            ),
            examples(
                datetime(2013, 2, 14, 6, 0, 0, Grain::Minute),
                vec![
                    "Thursday 8:00 GMT",
                    "Thursday 8:00 gmt",
                    "Thursday 8h00 GMT",
                    "Thursday 8h00 gmt",
                    "Thursday 8h GMT",
                    "Thursday 8h gmt",
                    "Thu at 8 GMT",
                    "Thu at 8 gmt",
                    "Thursday 9 am BST",
                    "Thursday 9 am (BST)",
                ],
            ),
            examples(
                datetime(2013, 2, 14, 14, 0, 0, Grain::Minute),
                vec![
                    "Thursday 8:00 PST",
                    "Thursday 8:00 pst",
                    "Thursday 8h00 PST",
                    "Thursday 8h00 pst",
                    "Thursday 8h PST",
                    "Thursday 8h pst",
                    "Thu at 8 am PST",
                    "Thu at 8 am pst",
                    "Thursday at 9:30pm ist",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 14, 0, 0, Grain::Hour),
                vec![
                    "today at 2pm",
                    "at 2pm",
                    "this afternoon at 2",
                    "this evening at 2",
                    "tonight at 2",
                ],
            ),
            examples(
                datetime(2013, 2, 13, 15, 0, 0, Grain::Hour),
                vec!["3pm tomorrow"],
            ),
            examples(
                datetime(2013, 2, 12, 5, 30, 0, Grain::Minute),
                vec!["today in one hour"],
            ),
            examples(
                datetime_open_interval_after(2013, 2, 12, 4, 30, 0, Grain::Second),
                vec!["ASAP", "as soon as possible"],
            ),
            examples(
                datetime_open_interval_before(2013, 2, 12, 14, 0, 0, Grain::Minute),
                vec!["until 2:00pm", "through 2:00pm"],
            ),
            examples(
                datetime_open_interval_after(2013, 2, 12, 14, 0, 0, Grain::Hour),
                vec!["after 2 pm", "from 2 pm", "since 2pm"],
            ),
            examples(
                datetime_open_interval_after(2014, 1, 1, 0, 0, 0, Grain::Year),
                vec!["anytime after 2014", "since 2014"],
            ),
            examples(
                datetime_open_interval_before(2014, 1, 1, 0, 0, 0, Grain::Year),
                vec!["sometimes before 2014", "through 2014"],
            ),
            examples(
                datetime_open_interval_after(2013, 2, 17, 4, 0, 0, Grain::Hour),
                vec!["after 5 days"],
            ),
            examples(
                datetime_open_interval_before(2013, 2, 12, 11, 0, 0, Grain::Hour),
                vec!["before 11 am"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 12, 0, 0, 2013, 2, 12, 19, 0, 0, Grain::Hour),
                vec!["in the afternoon"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 8, 0, 0, 2013, 2, 12, 19, 0, 0, Grain::Hour),
                vec!["8am until 6"],
            ),
            examples(
                datetime(2013, 2, 12, 13, 30, 0, Grain::Minute),
                vec!["at 1:30pm", "1:30pm", "at 13h30", "13h30"],
            ),
            examples(
                datetime(2013, 2, 12, 4, 45, 0, Grain::Second),
                vec!["in 15 minutes", "in 15'", "in 15"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 13, 0, 0, 2013, 2, 12, 17, 0, 0, Grain::Hour),
                vec!["after lunch"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 15, 0, 0, 2013, 2, 12, 21, 0, 0, Grain::Hour),
                vec!["after school"],
            ),
            examples(
                datetime(2013, 2, 12, 10, 30, 0, Grain::Minute),
                vec!["10:30", "approximately 1030"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 0, 0, 0, 2013, 2, 12, 12, 0, 0, Grain::Hour),
                vec!["this morning"],
            ),
            examples(
                datetime(2013, 2, 18, 0, 0, 0, Grain::Day),
                vec!["next monday"],
            ),
            examples(
                datetime(2013, 2, 12, 12, 0, 0, Grain::Hour),
                vec!["at 12pm", "at noon", "midday", "the midday", "mid day"],
            ),
            examples(
                datetime(2013, 2, 13, 0, 0, 0, Grain::Hour),
                vec![
                    "at 12am",
                    "at midnight",
                    "this morning at 12",
                    "this evening at 12",
                    "this afternoon at 12",
                ],
            ),
            examples(
                datetime(2013, 2, 13, 9, 0, 0, Grain::Hour),
                vec!["9 tomorrow morning", "9 tomorrow"],
            ),
            examples(
                datetime(2013, 2, 13, 21, 0, 0, Grain::Hour),
                vec!["9 tomorrow evening"],
            ),
            examples(
                datetime(2013, 3, 1, 0, 0, 0, Grain::Month),
                vec!["March", "in March", "during March"],
            ),
            examples(
                datetime(2013, 2, 13, 17, 0, 0, Grain::Hour),
                vec![
                    "tomorrow afternoon at 5",
                    "at 5 tomorrow afternoon",
                    "at 5pm tomorrow",
                    "tomorrow at 5pm",
                    "tomorrow evening at 5",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 13, 12, 0, 0, 2013, 2, 13, 19, 0, 0, Grain::Hour),
                vec!["tomorrow afternoon", "tomorrow afternoonish"],
            ),
            examples(
                datetime_interval(2013, 2, 13, 13, 0, 0, 2013, 2, 13, 15, 0, 0, Grain::Hour),
                vec!["1pm-2pm tomorrow"],
            ),
            examples(
                datetime(2013, 3, 1, 0, 0, 0, Grain::Day),
                vec!["on the first", "the 1st"],
            ),
            examples(
                datetime(2013, 2, 12, 10, 30, 0, Grain::Minute),
                vec!["at 1030", "around 1030", "ten thirty am"],
            ),
            examples(
                datetime(2013, 2, 12, 19, 30, 0, Grain::Minute),
                vec!["at 730 in the evening", "seven thirty p.m."],
            ),
            examples(
                datetime(2013, 2, 13, 1, 50, 0, Grain::Minute),
                vec!["tomorrow at 150ish"],
            ),
            examples(
                datetime(2013, 2, 12, 23, 0, 0, Grain::Hour),
                vec![
                    "tonight at 11",
                    "this evening at 11",
                    "this afternoon at 11",
                    "tonight at 11pm",
                ],
            ),
            examples(
                datetime(2013, 2, 12, 4, 23, 0, Grain::Minute),
                // yes, the result is in the past, we may need to revisit
                vec!["at 4:23", "4:23am", "four twenty-three a m"],
            ),
            examples(
                datetime(2013, 10, 7, 0, 0, 0, Grain::Day),
                vec!["the closest Monday to Oct 5th"],
            ),
            examples(
                datetime(2013, 9, 30, 0, 0, 0, Grain::Day),
                vec!["the second closest Mon to October fifth"],
            ),
            examples(
                datetime_interval(2013, 3, 1, 0, 0, 0, 2013, 3, 11, 0, 0, 0, Grain::Day),
                vec!["early March"],
            ),
            examples(
                datetime_interval(2013, 3, 11, 0, 0, 0, 2013, 3, 21, 0, 0, 0, Grain::Day),
                vec!["mid March"],
            ),
            examples(
                datetime_interval(2013, 3, 21, 0, 0, 0, 2013, 4, 1, 0, 0, 0, Grain::Day),
                vec!["late March"],
            ),
            examples(
                datetime_interval(2013, 10, 25, 18, 0, 0, 2013, 10, 28, 0, 0, 0, Grain::Hour),
                vec![
                    "last weekend of October",
                    "last week-end in October",
                    "last week end of October",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 11, 0, 0, 0, 2013, 2, 17, 0, 0, 0, Grain::Day),
                vec!["all week"],
            ),
            examples(
                datetime_interval(2013, 2, 12, 0, 0, 0, 2013, 2, 17, 0, 0, 0, Grain::Day),
                vec!["rest of the week"],
            ),
            examples(
                datetime_interval(2013, 7, 26, 18, 0, 0, 2013, 7, 29, 0, 0, 0, Grain::Hour),
                vec!["last wkend of July"],
            ),
            examples(
                datetime_interval(2017, 10, 27, 18, 0, 0, 2017, 10, 30, 0, 0, 0, Grain::Hour),
                vec!["last weekend of October 2017"],
            ),
            examples(
                datetime_interval(2013, 8, 27, 0, 0, 0, 2013, 8, 30, 0, 0, 0, Grain::Day),
                vec!["August 27th - 29th", "from August 27th - 29th"],
            ),
            examples(
                datetime_interval(2013, 10, 23, 0, 0, 0, 2013, 10, 27, 0, 0, 0, Grain::Day),
                vec!["23rd to 26th Oct"],
            ),
            examples(
                datetime_interval(2013, 9, 1, 0, 0, 0, 2013, 9, 9, 0, 0, 0, Grain::Day),
                vec!["1-8 september"],
            ),
            examples(
                datetime_interval(2013, 9, 12, 0, 0, 0, 2013, 9, 17, 0, 0, 0, Grain::Day),
                vec!["12 to 16 september"],
            ),
            examples(
                datetime_interval(2013, 8, 19, 0, 0, 0, 2013, 8, 22, 0, 0, 0, Grain::Day),
                vec!["19th To 21st aug"],
            ),
            examples(
                datetime_interval(2013, 4, 21, 0, 0, 0, 2013, 5, 1, 0, 0, 0, Grain::Day),
                vec!["end of April", "at the end of April"],
            ),
            examples(
                datetime_interval(2014, 1, 1, 0, 0, 0, 2014, 1, 11, 0, 0, 0, Grain::Day),
                vec!["beginning of January", "at the beginning of January"],
            ),
            examples(
                datetime_interval(2012, 9, 1, 0, 0, 0, 2013, 1, 1, 0, 0, 0, Grain::Month),
                vec!["end of 2012", "at the end of 2012"],
            ),
            examples(
                datetime_interval(2017, 1, 1, 0, 0, 0, 2017, 4, 1, 0, 0, 0, Grain::Month),
                vec!["beginning of 2017", "at the beginning of 2017"],
            ),
            examples(
                datetime_interval(2013, 1, 1, 0, 0, 0, 2013, 4, 1, 0, 0, 0, Grain::Month),
                vec![
                    "beginning of year",
                    "the beginning of the year",
                    "the BOY",
                    "BOY",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 12, 4, 30, 0, 2014, 1, 1, 0, 0, 0, Grain::Second),
                vec![
                    "by EOY",
                    "by the EOY",
                    "by end of the year",
                    "by the end of year",
                ],
            ),
            examples(
                datetime_interval(2013, 9, 1, 0, 0, 0, 2014, 1, 1, 0, 0, 0, Grain::Month),
                vec![
                    "EOY",
                    "the EOY",
                    "at the EOY",
                    "the end of the year",
                    "end of the year",
                    "at the end of year",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 11, 0, 0, 0, 2013, 2, 14, 0, 0, 0, Grain::Day),
                vec![
                    "beginning of this week",
                    "beginning of current week",
                    "at the beginning of this week",
                    "at the beginning of current week",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 18, 0, 0, 0, 2013, 2, 21, 0, 0, 0, Grain::Day),
                vec![
                    "beginning of coming week",
                    "at the beginning of coming week",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 4, 0, 0, 0, 2013, 2, 7, 0, 0, 0, Grain::Day),
                vec![
                    "beginning of last week",
                    "beginning of past week",
                    "beginning of previous week",
                    "at the beginning of last week",
                    "at the beginning of past week",
                    "at the beginning of previous week",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 18, 0, 0, 0, 2013, 2, 21, 0, 0, 0, Grain::Day),
                vec![
                    "beginning of next week",
                    "beginning of the following week",
                    "beginning of around next week",
                    "at the beginning of next week",
                    "at the beginning of the following week",
                    "at the beginning of around next week",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 15, 0, 0, 0, 2013, 2, 18, 0, 0, 0, Grain::Day),
                vec![
                    "end of this week",
                    "end of current week",
                    "at the end of this week",
                    "at the end of current week",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 22, 0, 0, 0, 2013, 2, 25, 0, 0, 0, Grain::Day),
                vec!["end of coming week", "at the end of coming week"],
            ),
            examples(
                datetime_interval(2013, 2, 8, 0, 0, 0, 2013, 2, 11, 0, 0, 0, Grain::Day),
                vec![
                    "end of last week",
                    "end of past week",
                    "end of previous week",
                    "at the end of last week",
                    "at the end of past week",
                    "at the end of previous week",
                ],
            ),
            examples(
                datetime_interval(2013, 2, 22, 0, 0, 0, 2013, 2, 25, 0, 0, 0, Grain::Day),
                vec![
                    "end of next week",
                    "end of the following week",
                    "end of around next week",
                    "at the end of next week",
                    "at the end of the following week",
                    "at the end of around next week",
                ],
            ),
            examples(
                datetime_holiday(2014, 1, 31, 0, 0, 0, Grain::Day, "Chinese New Year"),
                vec!["chinese new year", "chinese lunar new year's day"],
            ),
            examples(
                datetime_holiday(2013, 2, 10, 0, 0, 0, Grain::Day, "Chinese New Year"),
                vec![
                    "last chinese new year",
                    "last chinese lunar new year's day",
                    "last chinese new years",
                ],
            ),
            examples(
                datetime_holiday(2018, 2, 16, 0, 0, 0, Grain::Day, "Chinese New Year"),
                vec!["chinese new year's day 2018"],
            ),
            examples(
                datetime_holiday(2018, 9, 18, 0, 0, 0, Grain::Day, "Yom Kippur"),
                vec!["yom kippur 2018"],
            ),
            examples(
                datetime_holiday(2018, 9, 30, 0, 0, 0, Grain::Day, "Shemini Atzeret"),
                vec!["shemini atzeret 2018"],
            ),
            examples(
                datetime_holiday(2018, 10, 1, 0, 0, 0, Grain::Day, "Simchat Torah"),
                vec!["simchat torah 2018"],
            ),
            examples(
                datetime_holiday(2018, 7, 21, 0, 0, 0, Grain::Day, "Tisha B'Av"),
                vec!["tisha b'av 2018"],
            ),
            examples(
                datetime_holiday(2018, 4, 18, 0, 0, 0, Grain::Day, "Yom Ha'atzmaut"),
                vec!["yom haatzmaut 2018"],
            ),
            examples(
                datetime_holiday(2017, 5, 13, 0, 0, 0, Grain::Day, "Lag BaOmer"),
                vec!["lag b'omer 2017"],
            ),
            examples(
                datetime_holiday(2018, 4, 11, 0, 0, 0, Grain::Day, "Yom HaShoah"),
                vec!["Yom Hashoah 2018", "Holocaust Day 2018"],
            ),
            examples(
                datetime_interval_holiday(
                    2018,
                    9,
                    9,
                    0,
                    0,
                    0,
                    2018,
                    9,
                    12,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Rosh Hashanah",
                ),
                vec![
                    "rosh hashanah 2018",
                    "rosh hashana 2018",
                    "rosh hashanna 2018",
                ],
            ),
            examples(
                datetime_interval_holiday(
                    2018,
                    12,
                    2,
                    0,
                    0,
                    0,
                    2018,
                    12,
                    10,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Hanukkah",
                ),
                vec!["Chanukah 2018", "hanukah 2018", "hannukkah 2018"],
            ),
            examples(
                datetime_interval_holiday(
                    2018,
                    3,
                    30,
                    0,
                    0,
                    0,
                    2018,
                    4,
                    8,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Passover",
                ),
                vec!["passover 2018"],
            ),
            examples(
                datetime_interval_holiday(
                    2018,
                    9,
                    23,
                    0,
                    0,
                    0,
                    2018,
                    10,
                    2,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Sukkot",
                ),
                vec!["feast of the ingathering 2018", "succos 2018"],
            ),
            examples(
                datetime_interval_holiday(
                    2018,
                    5,
                    19,
                    0,
                    0,
                    0,
                    2018,
                    5,
                    22,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Shavuot",
                ),
                vec!["shavuot 2018"],
            ),
            examples(
                datetime_holiday(2017, 11, 30, 0, 0, 0, Grain::Day, "Mawlid"),
                vec!["mawlid al-nabawi 2017"],
            ),
            examples(
                datetime_holiday(1950, 7, 16, 0, 0, 0, Grain::Day, "Eid al-Fitr"),
                vec!["Eid al-Fitr 1950"],
            ),
            examples(
                datetime_holiday(1975, 10, 6, 0, 0, 0, Grain::Day, "Eid al-Fitr"),
                vec!["Eid al-Fitr 1975"],
            ),
            examples(
                datetime_holiday(1988, 5, 16, 0, 0, 0, Grain::Day, "Eid al-Fitr"),
                vec!["Eid al-Fitr 1988"],
            ),
            examples(
                datetime_holiday(2018, 6, 15, 0, 0, 0, Grain::Day, "Eid al-Fitr"),
                vec!["Eid al-Fitr 2018"],
            ),
            examples(
                datetime_holiday(2034, 12, 12, 0, 0, 0, Grain::Day, "Eid al-Fitr"),
                vec!["Eid al-Fitr 2034"],
            ),
            examples(
                datetime_holiday(2046, 8, 4, 0, 0, 0, Grain::Day, "Eid al-Fitr"),
                vec!["Eid al-Fitr 2046"],
            ),
            examples(
                datetime_holiday(2050, 6, 21, 0, 0, 0, Grain::Day, "Eid al-Fitr"),
                vec!["Eid al-Fitr 2050"],
            ),
            examples(
                datetime_holiday(2018, 8, 21, 0, 0, 0, Grain::Day, "Eid al-Adha"),
                vec![
                    "Eid al-Adha 2018",
                    "id ul-adha 2018",
                    "sacrifice feast 2018",
                    "Bakr Id 2018",
                ],
            ),
            examples(
                datetime_holiday(1980, 10, 19, 0, 0, 0, Grain::Day, "Eid al-Adha"),
                vec![
                    "Eid al-Adha 1980",
                    "id ul-adha 1980",
                    "sacrifice feast 1980",
                    "Bakr Id 1980",
                ],
            ),
            examples(
                datetime_holiday(1966, 4, 1, 0, 0, 0, Grain::Day, "Eid al-Adha"),
                vec![
                    "Eid al-Adha 1966",
                    "id ul-adha 1966",
                    "sacrifice feast 1966",
                    "Bakr Id 1966",
                ],
            ),
            examples(
                datetime_holiday(1974, 1, 3, 0, 0, 0, Grain::Day, "Eid al-Adha"),
                vec![
                    "Eid al-Adha 1974",
                    "id ul-adha 1974",
                    "sacrifice feast 1974",
                    "Bakr Id 1974",
                ],
            ),
            examples(
                datetime_holiday(2017, 6, 22, 0, 0, 0, Grain::Day, "Laylat al-Qadr"),
                vec!["laylat al kadr 2017", "night of measures 2017"],
            ),
            examples(
                datetime_holiday(2018, 6, 11, 0, 0, 0, Grain::Day, "Laylat al-Qadr"),
                vec!["laylat al-qadr 2018", "night of power 2018"],
            ),
            examples(
                datetime_holiday(2018, 9, 11, 0, 0, 0, Grain::Day, "Islamic New Year"),
                vec!["Islamic New Year 2018", "Amun Jadid 2018"],
            ),
            examples(
                datetime_holiday(2017, 9, 30, 0, 0, 0, Grain::Day, "Ashura"),
                vec!["day of Ashura 2017"],
            ),
            examples(
                datetime_holiday(2018, 1, 30, 0, 0, 0, Grain::Day, "Tu BiShvat"),
                vec!["tu bishvat 2018"],
            ),
            examples(
                datetime_holiday(2017, 6, 23, 0, 0, 0, Grain::Day, "Jumu'atul-Wida"),
                vec!["Jamat Ul-Vida 2017", "Jumu'atul-Wida 2017"],
            ),
            examples(
                datetime_holiday(2018, 6, 8, 0, 0, 0, Grain::Day, "Jumu'atul-Wida"),
                vec!["Jamat Ul-Vida 2018", "Jumu'atul-Wida 2018"],
            ),
            examples(
                datetime_holiday(2018, 4, 13, 0, 0, 0, Grain::Day, "Isra and Mi'raj"),
                vec!["isra and mi'raj 2018", "the prophet's ascension 2018"],
            ),
            examples(
                datetime_holiday(2019, 4, 3, 0, 0, 0, Grain::Day, "Isra and Mi'raj"),
                vec!["the night journey 2019", "ascension to heaven 2019"],
            ),
            examples(
                datetime_interval_holiday(
                    1950,
                    6,
                    17,
                    0,
                    0,
                    0,
                    1950,
                    7,
                    16,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Ramadan",
                ),
                vec!["Ramadan 1950"],
            ),
            examples(
                datetime_interval_holiday(
                    1977,
                    8,
                    15,
                    0,
                    0,
                    0,
                    1977,
                    9,
                    14,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Ramadan",
                ),
                vec!["Ramadan 1977"],
            ),
            examples(
                datetime_interval_holiday(
                    2018,
                    5,
                    16,
                    0,
                    0,
                    0,
                    2018,
                    6,
                    15,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Ramadan",
                ),
                vec!["Ramadan 2018"],
            ),
            examples(
                datetime_interval_holiday(
                    2034,
                    11,
                    12,
                    0,
                    0,
                    0,
                    2034,
                    12,
                    12,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Ramadan",
                ),
                vec!["Ramadan 2034"],
            ),
            examples(
                datetime_interval_holiday(
                    2046,
                    7,
                    5,
                    0,
                    0,
                    0,
                    2046,
                    8,
                    4,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Ramadan",
                ),
                vec!["Ramadan 2046"],
            ),
            examples(
                datetime_interval_holiday(
                    2050,
                    5,
                    22,
                    0,
                    0,
                    0,
                    2050,
                    6,
                    21,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Ramadan",
                ),
                vec!["Ramadan 2050"],
            ),
            examples(
                datetime_holiday(2017, 10, 17, 0, 0, 0, Grain::Day, "Dhanteras"),
                vec!["dhanatrayodashi in 2017"],
            ),
            examples(
                datetime_holiday(2019, 10, 25, 0, 0, 0, Grain::Day, "Dhanteras"),
                vec!["dhanteras 2019"],
            ),
            examples(
                datetime_holiday(2019, 10, 26, 0, 0, 0, Grain::Day, "Naraka Chaturdashi"),
                vec!["kali chaudas 2019", "choti diwali two thousand nineteen"],
            ),
            examples(
                datetime_holiday(2019, 10, 27, 0, 0, 0, Grain::Day, "Diwali"),
                vec![
                    "diwali 2019",
                    "Deepavali in 2019",
                    "Lakshmi Puja six years hence",
                ],
            ),
            examples(
                datetime_holiday(2019, 10, 29, 0, 0, 0, Grain::Day, "Bhai Dooj"),
                vec!["bhai dooj 2019"],
            ),
            examples(
                datetime_holiday(2019, 11, 2, 0, 0, 0, Grain::Day, "Chhath"),
                vec!["chhath 2019", "dala puja 2019", "Surya Shashthi in 2019"],
            ),
            examples(
                datetime_holiday(2021, 10, 12, 0, 0, 0, Grain::Day, "Maha Saptami"),
                vec!["Maha Saptami 2021"],
            ),
            examples(
                datetime_holiday(2018, 10, 18, 0, 0, 0, Grain::Day, "Vijayadashami"),
                vec!["Dussehra 2018", "vijayadashami in five years"],
            ),
            examples(
                datetime_interval_holiday(
                    2018,
                    10,
                    9,
                    0,
                    0,
                    0,
                    2018,
                    10,
                    19,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Navaratri",
                ),
                vec!["navaratri 2018", "durga puja in 2018"],
            ),
            examples(
                datetime_holiday(2018, 10, 27, 0, 0, 0, Grain::Day, "Karva Chauth"),
                vec!["karva chauth 2018", "karva chauth in 2018"],
            ),
            examples(
                datetime_holiday(2018, 7, 14, 0, 0, 0, Grain::Day, "Ratha-Yatra"),
                vec!["ratha-yatra 2018"],
            ),
            examples(
                datetime_holiday(2018, 8, 26, 0, 0, 0, Grain::Day, "Raksha Bandhan"),
                vec!["rakhi 2018"],
            ),
            examples(
                datetime_holiday(2020, 4, 6, 0, 0, 0, Grain::Day, "Mahavir Jayanti"),
                vec!["mahavir jayanti 2020", "mahaveer janma kalyanak 2020"],
            ),
            examples(
                datetime_holiday(2020, 2, 21, 0, 0, 0, Grain::Day, "Maha Shivaratri"),
                vec!["maha shivaratri 2020"],
            ),
            examples(
                datetime_holiday(
                    2018,
                    2,
                    10,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Dayananda Saraswati Jayanti",
                ),
                vec!["saraswati jayanti 2018"],
            ),
            examples(
                datetime_holiday(2018, 1, 14, 0, 0, 0, Grain::Day, "Thai Pongal"),
                vec!["pongal 2018", "makara sankranthi 2018"],
            ),
            examples(
                datetime_holiday(2018, 1, 13, 0, 0, 0, Grain::Day, "Boghi"),
                vec!["bogi pandigai 2018"],
            ),
            examples(
                datetime_holiday(2018, 1, 15, 0, 0, 0, Grain::Day, "Mattu Pongal"),
                vec!["maattu pongal 2018"],
            ),
            examples(
                datetime_holiday(2018, 1, 16, 0, 0, 0, Grain::Day, "Kaanum Pongal"),
                vec!["kaanum pongal 2018", "kanni pongal 2018"],
            ),
            examples(
                datetime_holiday(2019, 1, 15, 0, 0, 0, Grain::Day, "Thai Pongal"),
                vec!["makar sankranti 2019", "maghi in 2019"],
            ),
            examples(
                datetime_holiday(2018, 4, 14, 0, 0, 0, Grain::Day, "Vaisakhi"),
                vec![
                    "Vaisakhi 2018",
                    "baisakhi in 2018",
                    "Vasakhi 2018",
                    "vaishakhi 2018",
                ],
            ),
            examples(
                datetime_holiday(2018, 8, 24, 0, 0, 0, Grain::Day, "Thiru Onam"),
                vec!["onam 2018", "Thiru Onam 2018", "Thiruvonam 2018"],
            ),
            examples(
                datetime_holiday(2019, 2, 10, 0, 0, 0, Grain::Day, "Vasant Panchami"),
                vec!["vasant panchami in 2019", "basant panchami 2019"],
            ),
            examples(
                datetime_holiday(2019, 3, 20, 0, 0, 0, Grain::Day, "Holika Dahan"),
                vec!["chhoti holi 2019", "holika dahan 2019", "kamudu pyre 2019"],
            ),
            examples(
                datetime_holiday(2019, 8, 23, 0, 0, 0, Grain::Day, "Krishna Janmashtami"),
                vec!["krishna janmashtami 2019", "gokulashtami 2019"],
            ),
            examples(
                datetime_holiday(2019, 3, 21, 0, 0, 0, Grain::Day, "Holi"),
                vec!["holi 2019", "dhulandi 2019", "phagwah 2019"],
            ),
            examples(
                datetime_holiday(2018, 8, 17, 0, 0, 0, Grain::Day, "Parsi New Year"),
                vec!["Parsi New Year 2018", "Jamshedi Navroz 2018"],
            ),
            examples(
                datetime_holiday(2022, 8, 16, 0, 0, 0, Grain::Day, "Parsi New Year"),
                vec!["jamshedi Navroz 2022", "parsi new year 2022"],
            ),
            examples(
                datetime_interval_holiday(
                    2013,
                    4,
                    26,
                    0,
                    0,
                    0,
                    2013,
                    4,
                    29,
                    0,
                    0,
                    0,
                    Grain::Day,
                    "Global Youth Service Day",
                ),
                vec!["GYSD 2013", "global youth service day"],
            ),
            examples(
                datetime_holiday(2013, 5, 24, 0, 0, 0, Grain::Day, "Vesak"),
                vec!["vesak", "vaisakha", "Buddha day", "Buddha Purnima"],
            ),
            examples(
                datetime_interval_holiday(
                    2013,
                    3,
                    23,
                    20,
                    30,
                    0,
                    2013,
                    3,
                    23,
                    21,
                    31,
                    0,
                    Grain::Minute,
                    "Earth Hour",
                ),
                vec!["earth hour"],
            ),
            examples(
                datetime_interval_holiday(
                    2016,
                    3,
                    19,
                    20,
                    30,
                    0,
                    2016,
                    3,
                    19,
                    21,
                    31,
                    0,
                    Grain::Minute,
                    "Earth Hour",
                ),
                vec!["earth hour 2016"],
            ),
            examples(
                datetime_holiday(2013, 2, 23, 0, 0, 0, Grain::Day, "Purim"),
                vec!["purim"],
            ),
            examples(
                datetime_holiday(2013, 2, 24, 0, 0, 0, Grain::Day, "Shushan Purim"),
                vec!["Shushan Purim"],
            ),
            examples(
                datetime_holiday(2014, 1, 7, 0, 0, 0, Grain::Day, "Guru Gobind Singh Jayanti"),
                vec![
                    "guru gobind singh birthday",
                    "guru gobind singh jayanti 2014",
                    "guru gobind singh jayanti",
                    "Guru Govind Singh Jayanti",
                ],
            ),
            examples(
                datetime_holiday(2018, 4, 27, 0, 0, 0, Grain::Day, "King's Day"),
                vec![
                    "Koningsdag 2018",
                    "koningsdag 2018",
                    "king's day 2018",
                    "King's Day 2018",
                ],
            ),
            examples(
                datetime_holiday(2014, 4, 26, 0, 0, 0, Grain::Day, "King's Day"),
                vec![
                    "Koningsdag 2014",
                    "koningsdag 2014",
                    "King's Day 2014",
                    "king's day 2014",
                ],
            ),
            examples(
                datetime_holiday(2018, 5, 9, 0, 0, 0, Grain::Day, "Rabindra Jayanti"),
                vec![
                    "rabindra jayanti 2018",
                    "Rabindranath Jayanti 2018",
                    "Rabindra Jayanti 2018",
                ],
            ),
            examples(
                datetime_holiday(2019, 5, 9, 0, 0, 0, Grain::Day, "Rabindra Jayanti"),
                vec![
                    "rabindra jayanti 2019",
                    "Rabindranath Jayanti 2019",
                    "Rabindra Jayanti 2019",
                ],
            ),
            examples(
                datetime_holiday(2018, 1, 31, 0, 0, 0, Grain::Day, "Guru Ravidass Jayanti"),
                vec![
                    "guru Ravidas jayanti 2018",
                    "Guru Ravidass birthday 2018",
                    "guru ravidass Jayanti 2018",
                ],
            ),
            examples(
                datetime_holiday(2019, 2, 19, 0, 0, 0, Grain::Day, "Guru Ravidass Jayanti"),
                vec![
                    "Guru Ravidass Jayanti 2019",
                    "Guru Ravidas Birthday 2019",
                    "guru ravidas jayanti 2019",
                ],
            ),
            examples(
                datetime_holiday(2019, 10, 13, 0, 0, 0, Grain::Day, "Pargat Diwas"),
                vec![
                    "valmiki jayanti 2019",
                    "Valmiki Jayanti 2019",
                    "pargat diwas 2019",
                ],
            ),
            examples(
                datetime_holiday(2018, 10, 24, 0, 0, 0, Grain::Day, "Pargat Diwas"),
                vec![
                    "maharishi valmiki jayanti 2018",
                    "pargat diwas 2018",
                    "Pargat Diwas 2018",
                ],
            ),
            examples(
                datetime_holiday(2019, 9, 2, 0, 0, 0, Grain::Day, "Ganesh Chaturthi"),
                vec!["ganesh chaturthi 2019"],
            ),
            examples(
                datetime_holiday(2020, 4, 2, 0, 0, 0, Grain::Day, "Rama Navami"),
                vec!["rama navami 2020"],
            ),
            examples(
                datetime_holiday(2018, 3, 18, 0, 0, 0, Grain::Day, "Ugadi"),
                vec![
                    "Ugadi 2018",
                    "ugadi 2018",
                    "yugadi 2018",
                    "Yugadi 2018",
                    "samvatsaradi 2018",
                    "chaitra sukladi 2018",
                    "chaitra sukhladi 2018",
                ],
            ),
            examples(
                datetime_holiday(2012, 12, 25, 0, 0, 0, Grain::Day, "Christmas"),
                vec!["the closest xmas to today"],
            ),
            examples(
                datetime_holiday(2013, 12, 25, 0, 0, 0, Grain::Day, "Christmas"),
                vec!["the second closest xmas to today"],
            ),
            examples(
                datetime_holiday(2011, 12, 25, 0, 0, 0, Grain::Day, "Christmas"),
                vec!["the 3rd closest xmas to today"],
            ),
            examples(
                datetime(2013, 10, 25, 0, 0, 0, Grain::Day),
                vec!["last friday of october", "last friday in october"],
            ),
            examples(
                datetime(2013, 2, 25, 0, 0, 0, Grain::Week),
                vec![
                    "upcoming two weeks",
                    "upcoming two week",
                    "upcoming 2 weeks",
                    "upcoming 2 week",
                    "two upcoming weeks",
                    "two upcoming week",
                    "2 upcoming weeks",
                    "2 upcoming week",
                ],
            ),
            examples(
                datetime(2013, 2, 14, 0, 0, 0, Grain::Day),
                vec![
                    "upcoming two days",
                    "upcoming two day",
                    "upcoming 2 days",
                    "upcoming 2 day",
                    "two upcoming days",
                    "two upcoming day",
                    "2 upcoming days",
                    "2 upcoming day",
                ],
            ),
            examples(
                datetime(2013, 4, 1, 0, 0, 0, Grain::Month),
                vec![
                    "upcoming two months",
                    "upcoming two month",
                    "upcoming 2 months",
                    "upcoming 2 month",
                    "two upcoming months",
                    "two upcoming month",
                    "2 upcoming months",
                    "2 upcoming month",
                ],
            ),
            examples(
                datetime(2013, 7, 1, 0, 0, 0, Grain::Quarter),
                vec![
                    "upcoming two quarters",
                    "upcoming two quarter",
                    "upcoming 2 quarters",
                    "upcoming 2 quarter",
                    "two upcoming quarters",
                    "two upcoming quarter",
                    "2 upcoming quarters",
                    "2 upcoming quarter",
                ],
            ),
            examples(
                datetime(2015, 1, 1, 0, 0, 0, Grain::Year),
                vec![
                    "upcoming two years",
                    "upcoming two year",
                    "upcoming 2 years",
                    "upcoming 2 year",
                    "two upcoming years",
                    "two upcoming year",
                    "2 upcoming years",
                    "2 upcoming year",
                ],
            ),
            examples(
                datetime(2013, 2, 13, 13, 40, 0, Grain::Minute),
                vec!["20 minutes to 2pm tomorrow"],
            ),
            examples(
                datetime(2013, 1, 7, 0, 0, 0, Grain::Day),
                vec!["first monday of last month"],
            ),
            examples(
                datetime(2013, 1, 1, 0, 0, 0, Grain::Day),
                vec!["first tuesday of last month"],
            ),
            examples(
                datetime(2013, 1, 14, 0, 0, 0, Grain::Day),
                vec!["second monday of last month"],
            ),
            examples(
                datetime(2013, 2, 23, 0, 0, 0, Grain::Day),
                vec!["next saturday"],
            ),
            examples(
                datetime(2013, 2, 18, 0, 0, 0, Grain::Day),
                vec!["next monday"],
            ),
            // -- defaultCorpus custom examples (Thanksgiving etc.) --
            examples(
                datetime(2013, 2, 15, 0, 0, 0, Grain::Day),
                vec!["2/15", "on 2/15", "2 / 15", "2-15", "2 - 15"],
            ),
            examples(
                datetime(1974, 10, 31, 0, 0, 0, Grain::Day),
                vec![
                    "10/31/1974",
                    "10/31/74",
                    "10-31-74",
                    "10.31.1974",
                    "31/Oct/1974",
                    "31-Oct-74",
                    "31st Oct 1974",
                ],
            ),
            examples(
                datetime(2013, 4, 25, 16, 0, 0, Grain::Minute),
                vec!["4/25 at 4:00pm", "4/25 at 16h00", "4/25 at 16h"],
            ),
            examples(
                datetime_holiday(2013, 11, 28, 0, 0, 0, Grain::Day, "Thanksgiving Day"),
                vec![
                    "thanksgiving day",
                    "thanksgiving",
                    "thanksgiving 2013",
                    "this thanksgiving",
                    "next thanksgiving day",
                    "thanksgiving in 9 months",
                    "thanksgiving 9 months from now",
                ],
            ),
            examples(
                datetime_holiday(2014, 11, 27, 0, 0, 0, Grain::Day, "Thanksgiving Day"),
                vec![
                    "thanksgiving of next year",
                    "thanksgiving in a year",
                    "thanksgiving 2014",
                ],
            ),
            examples(
                datetime_holiday(2012, 11, 22, 0, 0, 0, Grain::Day, "Thanksgiving Day"),
                vec![
                    "last thanksgiving",
                    "thanksgiving day 2012",
                    "thanksgiving 3 months ago",
                    "thanksgiving 1 year ago",
                ],
            ),
            examples(
                datetime_holiday(2016, 11, 24, 0, 0, 0, Grain::Day, "Thanksgiving Day"),
                vec!["thanksgiving 2016", "thanksgiving in 3 years"],
            ),
            examples(
                datetime_holiday(2017, 11, 23, 0, 0, 0, Grain::Day, "Thanksgiving Day"),
                vec!["thanksgiving 2017"],
            ),
        ],
    )
}
