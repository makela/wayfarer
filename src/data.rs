use chrono::{NaiveDate, NaiveDateTime};
use maud::{html, Markup};
use rss::Item;
use std::collections::BTreeMap;

pub struct Post {
    pub title: String,
    pub url: String,
    pub published_at: NaiveDateTime,
}

impl From<&Post> for Item {
    fn from(post: &Post) -> Self {
        Item {
            title: Some(post.title.clone()),
            link: Some(format!("https://www.taichiwayfarer.com/post/{}", post.url)),
            description: None,
            author: None,
            categories: Vec::new(),
            comments: None,
            enclosure: None,
            guid: None,
            pub_date: Some(post.published_at.format("%a, %d %b %Y %T EET").to_string()),
            source: None,
            content: None,
            extensions: BTreeMap::new(),
            itunes_ext: None,
            dublin_core_ext: None,
        }
    }
}

pub struct PostLink {
    pub title: String,
    pub url: String,
}

#[expect(clippy::too_many_lines)]
pub fn get_posts() -> Vec<Post> {
    // Note: Last post is first, so prepend new stories!
    vec![
        /*
        Post {
            title: "The Sword Set".into(),
            url: "the-sword-set".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 8, 8)
                .unwrap_or_default()
                .and_hms_opt(8, 8, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Chinese Glossary".into(),
            url: "chinese-glossary".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 8, 8)
                .unwrap_or_default()
                .and_hms_opt(8, 8, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "The Slow Set".into(),
            url: "the-slow-set".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 8, 8)
                .unwrap_or_default()
                .and_hms_opt(8, 8, 0)
                .unwrap_or_default(),
        },
        */
        Post {
            title: "Come to the Center".into(),
            url: "june-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 7, 5)
                .unwrap_or_default()
                .and_hms_opt(22, 26, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Be Aware of Your Body".into(),
            url: "etna-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 5, 29)
                .unwrap_or_default()
                .and_hms_opt(13, 17, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Remember by Yourself".into(),
            url: "april-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 4, 30)
                .unwrap_or_default()
                .and_hms_opt(16, 34, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Take Your Time".into(),
            url: "march-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 3, 31)
                .unwrap_or_default()
                .and_hms_opt(16, 21, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Be a Shadow".into(),
            url: "february-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 2, 28)
                .unwrap_or_default()
                .and_hms_opt(12, 32, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Do Your Own Way".into(),
            url: "january-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 2, 11)
                .unwrap_or_default()
                .and_hms_opt(14, 27, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Use Your Mind".into(),
            url: "december-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 12, 14)
                .unwrap_or_default()
                .and_hms_opt(11, 44, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Practice On Your Own".into(),
            url: "november-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 12, 3)
                .unwrap_or_default()
                .and_hms_opt(18, 28, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Don't Stop".into(),
            url: "october-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 10, 20)
                .unwrap_or_default()
                .and_hms_opt(17, 55, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Do Bigger".into(),
            url: "sandy-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 9, 23)
                .unwrap_or_default()
                .and_hms_opt(14, 5, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Always Practice".into(),
            url: "la-honda-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 9, 13)
                .unwrap_or_default()
                .and_hms_opt(18, 8, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Together".into(),
            url: "mera-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 7, 2)
                .unwrap_or_default()
                .and_hms_opt(10, 44, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Don't Do Anything Extra".into(),
            url: "lapland-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 6, 25)
                .unwrap_or_default()
                .and_hms_opt(20, 25, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Raise Your Spirit".into(),
            url: "may-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 6, 20)
                .unwrap_or_default()
                .and_hms_opt(14, 22, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Don't Teach Too Much".into(),
            url: "april-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 4, 17)
                .unwrap_or_default()
                .and_hms_opt(14, 31, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Pay Attention".into(),
            url: "march-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 4, 8)
                .unwrap_or_default()
                .and_hms_opt(9, 45, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Match Your Energy".into(),
            url: "february-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 2, 13)
                .unwrap_or_default()
                .and_hms_opt(9, 45, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Know Your Space".into(),
            url: "january-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 1, 31)
                .unwrap_or_default()
                .and_hms_opt(17, 32, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "It's all about the Body".into(),
            url: "december-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 12, 2)
                .unwrap_or_default()
                .and_hms_opt(17, 46, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Play Tai Chi".into(),
            url: "november-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 11, 30)
                .unwrap_or_default()
                .and_hms_opt(22, 2, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Copenhagen, October 2024".into(),
            url: "october-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 10, 23)
                .unwrap_or_default()
                .and_hms_opt(17, 5, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Sandy Island 2024".into(),
            url: "sandy-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 9, 18)
                .unwrap_or_default()
                .and_hms_opt(15, 15, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Lapland and Mera 2024".into(),
            url: "june-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 6, 20)
                .unwrap_or_default()
                .and_hms_opt(12, 21, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Etna Camp 2024".into(),
            url: "etna-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 5, 22)
                .unwrap_or_default()
                .and_hms_opt(12, 2, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Mèze and Mariehamn, March 2024".into(),
            url: "march-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 4, 4)
                .unwrap_or_default()
                .and_hms_opt(18, 22, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Helsinki and Palermo, February 2024".into(),
            url: "february-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 2, 29)
                .unwrap_or_default()
                .and_hms_opt(21, 5, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Modesto and Vermont 2023".into(),
            url: "modesto-and-vermont-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 1, 6)
                .unwrap_or_default()
                .and_hms_opt(14, 45, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Sandy Island 2023".into(),
            url: "sandy-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 12, 12)
                .unwrap_or_default()
                .and_hms_opt(15, 0, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "European Tour 2023".into(),
            url: "november-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 11, 10)
                .unwrap_or_default()
                .and_hms_opt(16, 5, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Bosco Falconeria, Harvest of Catarratto".into(),
            url: "bosco-harvest-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 10, 27)
                .unwrap_or_default()
                .and_hms_opt(17, 15, 0)
                .unwrap_or_default(),
        },
    ]
}

fn name_table(names: &[(&str, &str, &str)]) -> Markup {
    html! {
        @for (i, (characters, pinyin, name)) in names.iter().enumerate() {
            div .index { (format!("{}.", i+1)) }
            div .characters { (characters) }
            div .pinyin { (pinyin) }
            div .english { (name) }
        }
    }
}

#[expect(clippy::too_many_lines)]
pub fn get_extra_markup(url: &str) -> Markup {
    if url == "./posts/the-slow-set.md" {
        let names = vec![
            ("预备式", "Yùbèi shì", "Preparation"),
            ("太极起式", "Tàijí qǐ shì", "Tai Chi Starting Movement"),
            ("揽雀尾", "Lǎn què wěi", "Embrace the Bird's Tail"),
            ("单鞭", "Dān biān", "Single Whip"),
            ("提手", "Tí shǒu", "Close Hands"),
            ("白鹤亮翅", "Báihè liàng chì", "White Crane Spreads Wings"),
            (
                "左搂膝拗步",
                "Zuǒ lǒu xī níu bù",
                "Brush Left Knee and Push",
            ),
            ("手挥琵琶", "Shǒu huī pípá", "Play the Guitar"),
            (
                "左搂膝拗步",
                "Zuǒ lǒu xī níu bù",
                "Brush Left Knee and Push",
            ),
            (
                "右搂膝拗步",
                "Yòu lǒu xī níu bù",
                "Brush Right Knee and Push",
            ),
            (
                "左搂膝拗步",
                "Zuǒ lǒu xī níu bù",
                "Brush Left Knee and Push",
            ),
            ("手挥琵琶", "Shǒu huī pípá", "Play the Guitar"),
            (
                "左搂膝拗步",
                "Zuǒ lǒu xī níu bù",
                "Brush Left Knee and Push",
            ),
            (
                "进步搬揽捶",
                "Jìn bù bān lǎn chuí",
                "Step Forward and Punch",
            ),
            ("如封似闭", "Rú fēng sì bì", "Retreat and Seal"),
            ("十字手", "Shí zì shǒu", "Cross Hands"),
            (
                "抱虎归山",
                "Bào hǔ guī shān",
                "Carry Tiger Back to Mountain",
            ),
            ("肘底看捶", "Zhǒu dǐ kàn chuí", "Fist Under Elbow"),
            ("倒撵猴", "Dào niǎn hóu", "Step Back and Repulse Monkey"),
            ("斜飞式", "Xié fēi shì", "Slanted Flying"),
            ("提手", "Tí shǒu", "Close Hands"),
            ("白鹤亮翅", "Báihè liàng chì", "White Crane Spreads Wings"),
            (
                "左搂膝拗步",
                "Zuǒ lǒu xī níu bù",
                "Brush Left Knee and Push",
            ),
            ("海底针", "Hǎi dǐ zhēn", "Needle at the Bottom of the Sea"),
            ("山通臂", "Shān tōng bì", "Raise Arms and Push"),
            ("抷身锤", "Pī shēn chuí", "Fist Near the Waist"),
            (
                "上步搬揽捶",
                "Shàng bù bān lǎn chuí",
                "Step Forward and Punch",
            ),
            ("揽雀尾", "Lǎn què wěi", "Embrace the Bird's Tail"),
            ("单鞭", "Dān biān", "Single Whip"),
            ("云手", "Yún shǒu", "Cloud Hands"),
            ("单鞭", "Dān biān", "Single Whip"),
            ("高探马", "Gāo tàn mǎ", "High-Pat the Horse"),
            ("右分脚", "Yòu fēn jiǎo", "Right Side Kick"),
            ("左分脚", "Zuǒ fèn jiǎo", "Left Side Kick"),
            (
                "转身蹬脚",
                "Zhuǎn shēn dēng jiǎo",
                "Turn Back and Heel Kick",
            ),
            (
                "左搂膝拗步",
                "Zuǒ lǒu xī níu bù",
                "Brush Left Knee and Push",
            ),
            (
                "右搂膝拗步",
                "Yòu lǒu xī níu bù",
                "Brush Right Knee and Push",
            ),
            (
                "进步搬揽捶",
                "Jìn bù bān lǎn chuí",
                "Step Forward and Punch Down",
            ),
            ("抷身锤", "Pī shēn chuí", "Fist Near the Waist"),
            (
                "上步搬揽捶",
                "Shàng bù bān lǎn chuí",
                "Step Forward and Punch",
            ),
            (
                "斜身右蹬脚",
                "Xié shēn yòu dēng jiǎo",
                "Twist Body and Right Heel Kick",
            ),
            ("打虎式", "Dǎ hǔ shì", "Strike the Tiger"),
            (
                "回身右蹬脚",
                "Huí shēn yòu dēng jiǎo",
                "Sit Back and Right Heel Kick",
            ),
            (
                "双凤貫耳",
                "Shuāng fèng guàn ěr",
                "Double Winds Through the Ears",
            ),
            ("左蹬脚", "Zuǒ dēng jiǎo", "Left Heel Kick"),
            (
                "转身右蹬脚",
                "Zhuǎn shēn yòu dēng jiǎo",
                "Turn Around and Right Heel Kick",
            ),
            (
                "上部搬揽捶",
                "Shàng bù bān lǎn chuí",
                "Step Forward and Punch",
            ),
            ("如封似闭", "Rú fēng sì bì", "Retreat and Seal"),
            ("十字手", "Shí zì shǒu", "Cross Hands"),
            (
                "抱虎归山",
                "Bào hǔ guī shān",
                "Carry Tiger Back to Mountain",
            ),
            ("斜单鞭", "Xié dān biān", "Diagonal Single Whip"),
            ("野马分鬃", "Yěmǎ fēn zōng", "Part the Wild Horse's Mane"),
            ("揽雀尾", "Lǎn què wěi", "Embrace the Bird's Tail"),
            ("单鞭", "Dān biān", "Single Whip"),
            ("玉女穿梭", "Yùnǚ chuān suō", "Fair Lady Works the Shuttle"),
            ("揽雀尾", "Lǎn què wěi", "Embrace the Bird's Tail"),
            ("单鞭", "Dān biān", "Single Whip"),
            ("云手", "Yún shǒu", "Cloud Hands"),
            ("单鞭", "Dān biān", "Single Whip"),
            ("单鞭下式", "Dān biān xià shì", "Single Whip Lower Posture"),
            (
                "金鸡独立",
                "Jīnjī dú lì",
                "Golden Rooster Stands on One Leg",
            ),
            ("倒撵猴", "Dào niǎn hóu", "Step Back and Repulse Monkey"),
            ("斜飞式", "Xié fēi shì", "Slanted Flying"),
            ("提手", "Tí shǒu", "Close Hands"),
            ("白鹤亮翅", "Báihè liàng chì", "White Crane Spreads Wings"),
            (
                "左搂膝拗步",
                "Zuǒ lǒu xī níu bù",
                "Brush Left Knee and Push",
            ),
            ("海底针", "Hǎi dǐ zhēn", "Needle at the Bottom of the Sea"),
            ("山通臂", "Shān tōng bì", "Raise Arms and Push"),
            ("白蛇吐信", "Báishé tǔ xìn", "White Snake Flicks Tongue"),
            (
                "上部搬揽捶",
                "Shàng bù bān lǎn chuí",
                "Step Forward and Punch",
            ),
            ("揽雀尾", "Lǎn què wěi", "Embrace the Bird's Tail"),
            ("单鞭", "Dān biān", "Single Whip"),
            ("云手", "Yún shǒu", "Cloud Hands"),
            ("单鞭", "Dān biān", "Single Whip"),
            ("高探马", "Gāo tàn mǎ", "High-Pat the Horse"),
            ("代穿掌", "Dài chuān zhǎng", "Thrust Out Palm"),
            (
                "转身十字腿",
                "Zhuǎn shēn shízì tuǐ",
                "Turn Around and Cross-Kick",
            ),
            (
                "进步指裆捶",
                "Jìnbù zhǐ dāng chuí",
                "Step Forward and Punch Groin",
            ),
            (
                "上步揽雀尾",
                "Shàng bù lǎn què wěi",
                "Embrace the Bird's Tail",
            ),
            ("单鞭", "Dān biān", "Single Whip"),
            ("单鞭下式", "Dān biān xià shì", "Single Whip Lower Posture"),
            (
                "上步七星捶",
                "Shàng bù qīxīng chuí",
                "Step Forward and Cross-Punch",
            ),
            ("退步跨虎", "Tuì bù kuà hǔ", "Step Back Over the Tiger"),
            (
                "转身双擺莲",
                "Zhuǎn shēn shuāng bǎi lián",
                "Turn Around and Slap-Kick",
            ),
            ("弯弓射虎", "Wān gōng shè hǔ", "Draw Bow and Shoot Tiger"),
            (
                "上部搬揽捶",
                "Shàng bù bān lǎn chuí",
                "Step Forward and Punch",
            ),
            ("如封似闭", "Rú fēng sì bì", "Retreat and Seal"),
            ("十字手", "Shí zì shǒu", "Cross Hands"),
            ("合太极", "Hé Tàijí", "Tai Chi Ending Movement"),
        ];
        html! {
            div #slow-set-video.video {}
            script async src="/static/embed-video.js" {}
            div .names-of-set {
                // 太極拳架子名目
                div .full { h3 { "Movements of the Slow Set" } }
                (name_table(&names))
            }
        }
    } else if url == "./posts/the-sword-set.md" {
        let names = vec![
            ("太极起式", "Tàijí qǐ shì", "Tai Chi Starting Movement"),
            (
                "三環套月",
                "Sān húan tào yùe",
                "Three Rings Around the Moon",
            ),
            ("魁星式", "Kuí xīng shì", "The Big Dipper"),
            ("燕子抄水", "Yàn zǐ chāo shuǐ", "Swallow Touching the Water"),
            (
                "左右攔掃",
                "Zuǒ yòu lán sǎo",
                "Side Sweeping, Left and Right",
            ),
            ("小魁星", "Xiǎo kuí xīng", "The Little Dipper"),
            (
                "燕子歸巢",
                "Yàn zǐ guī cháo",
                "Swallow Returning to the Nest",
            ),
            ("靈猫捕鼠", "Líng māo bǔ shǔ", "Cat Chasing the Mouse"),
            ("風凰抬头", "Fèng huáng tái tóu", "Phoenix Raising Her Head"),
            (
                "黄蜂入洞",
                "Huáng fēng rù dòng",
                "Wasp Flying into the Cave",
            ),
            (
                "鳳凰右展翅",
                "Fèng huáng yòu zhǎn chì",
                "Phoenix Spreading Her Right Wing",
            ),
            ("小魁星", "Xiǎo kuí xīng", "The Little Dipper"),
            (
                "鳳凰左展翅",
                "Fèng huáng zuǒ zhǎn chì",
                "Phoenix Spreading Her Left Wing",
            ),
            ("等魚式", "Děng yú shì", "Fishing Posture"),
            (
                "左右龍行",
                "Zuǒ yòu lóng xíng",
                "Dragon Walking, Left and Right",
            ),
            (
                "宿鳥投林",
                "Sù niǎo tóu lín",
                "Birds Returning to the Trees",
            ),
            ("烏龍擺尾", "Wū lóng bǎi wěi", "Dark Dragon Waving His Tail"),
            (
                "青龍出海",
                "Qīng lóng chū hǎi",
                "Dragon Coming Out of the Sea",
            ),
            ("風捲荷葉", "Fēng juǎn hé yè", "Wind Sweeping Lotus Leaves"),
            (
                "左右獅子搖頭",
                "Zuǒ yòu shīzǐ yáo tóu",
                "Lion Shaking His Head, Left and Right",
            ),
            ("虎抱頭", "Hǔ bào tóu", "Holding the Tiger's Head"),
            (
                "野馬跳潤",
                "Yě mǎ tiào jiàn",
                "Wild Horse Leaping Over the Creek",
            ),
            ("勒馬式", "Lè mă shì", "Reining the Horse"),
            ("指南針", "Zhǐ nán zhēn", "The Compass"),
            (
                "左右迎風揮塵",
                "Zuǒ yòu yíng fēng dǎn chén",
                "Dusting Against the Wind, Left and Right",
            ),
            ("顺水推舟", "Shùn shuǐ tuī zhōu", "Current Pushing the Boat"),
            ("流星趕月", "Liú xīng gǎn yuè", "Meteor Chasing the Moon"),
            (
                "天馬行空",
                "Tiān mǎ xíng kōng",
                "Fairy Horse Galloping in the Sky",
            ),
            ("挑簾式", "Tiāo lián shì", "Lifting the Curtain"),
            (
                "左右車輪",
                "Zuǒ yòu chē lún",
                "Wagon Wheeling, Left and Right",
            ),
            ("燕子御泥", "Yàn zǐ xián ní", "Swallow Carrying Dirt"),
            ("大鹏展翅", "Dà péng zhǎn chì", "Eagel Spreading His Wings"),
            (
                "海底撈月",
                "Hǎi dǐ lāo yuè",
                "Scooping the Moon from the Sea",
            ),
            ("懷中抱月", "Huái zhōng bào yuè", "Embracing the Moon"),
            (
                "哪吒探海",
                "Nǎ Zhā tàn hǎi",
                "Na Zha Venturing into the Sea",
            ),
            (
                "犀牛望月",
                "Xī niú wàng yuè",
                "Rhinoceros Gazing at the Moon",
            ),
            ("射雁式", "Shè yàn shì", "Shooting the Wild Goose"),
            (
                "青龍探爪",
                "Qīng lóng tàn zhǎo",
                "Dragon Stretching Her Wings",
            ),
            (
                "風凰雙展翅",
                "Fèng huáng shuāng zhǎn chì",
                "Phoenix Spreading Her Wings",
            ),
            (
                "左右挎攔",
                "Zuǒ yòu kuǎ lán",
                "Crossing the Fence, Left and Right",
            ),
            ("射雁式", "Shè yàn shì", "Shooting the Wild Goose"),
            ("白猿獻菜", "Bái yuán xiàn guǒ", "White Ape Offering Fruits"),
            (
                "左右落花",
                "Zuǒ yòu luò huā",
                "Falling Flowers, Left and Right",
            ),
            ("玉女穿梭", "Yùnǚ chuān suō", "Fair Lady Works the Shuttle"),
            (
                "白虎攪尾",
                "Bái hǔ jiǎo wěi",
                "White Tiger Raising His Tail",
            ),
            (
                "魚躍龍門",
                "Yú yǎo lóng mén",
                "Carp Jumping Over the Dragon Gate",
            ),
            (
                "左右烏龍絞柱",
                "Zuǒ yòu wū lóng jiǎo zhù",
                "Dark Dragon Wreathing Around the Pole",
            ),
            ("仙人指路", "Xiān rén zhǐ lù", "Fairy Pointing the Way"),
            (
                "朝天一指香",
                "Cháo tiān yī zhǐ xiāng",
                "Single Incense Pointing to the Sky",
            ),
            (
                "風掃梅花",
                "Fēng sǎo méi huā",
                "Wind Sweeping Plum Blossoms",
            ),
            ("牙筋式", "Yá hù shì", "Holding the Tablet"),
            ("合太极", "Hé Tàijí", "Tai Chi Ending Movement"),
        ];
        html! {
            div #sword-set-video.video {}
            script async src="/static/embed-video.js" {}
            div .names-of-set {
                div .full { h3 { "Movements of the Sword Set" } }
                (name_table(&names))
            }
        }
    } else if url == "./posts/chinese-glossary.md" {
        let terms = vec![
            (
                "Syllabus",
                vec![
                    ("太极拳", "Tàijíquán", "Tai Chi Chuan"),
                    ("慢拳", "Màn quán", "slow set"),
                    ("快拳", "Kuài quán", "fast set"),
                    ("董家", "Dǒng jiā", "Tung family set"),
                    ("太极刀", "Tàijí dāo", "Tai Chi knife"),
                    ("", "", "Tai Chi sticks"),
                    ("太极剑", "Tàijí jiàn", "Tai Chi sword"),
                    ("开合 | 開合", "Kāi hé", "opening and closing"),
                    ("对练", "Duì liàn", ""),
                    ("太极功", "Tàijí gōng", ""),
                    ("推手", "Tuīshǒu", "push hands"),
                    ("??", "shan shou", ""),
                    ("三步", "Sān bù", "three steps"),
                    ("大捋", "Dà lǚ", "four corners"),
                ],
            ),
            (
                "Mind and body",
                vec![
                    ("意", "yì", "intention"),
                    ("身", "shēn", "body"),
                    ("腰", "yāo", "waist"),
                    ("腿", "tuǐ", "leg"),
                    ("手", "shǒu", "hand"),
                    ("捶", "chuí", "fist"),
                    ("根部", "gēnbù", "roots"),
                ],
            ),
            (
                "Directions",
                vec![
                    ("左", "zuǒ", "left"),
                    ("右", "yòu", "right"),
                    ("下", "xià", "down"),
                ],
            ),
            (
                "Techniques",
                vec![
                    ("掤", "pèng", "ward off"),
                    ("捋", "lǚ", "roll back"),
                    ("擠", "jǐ", "press"),
                    ("按", "àn", "push"),
                    ("靠", "kào", "shoulder strike"),
                    ("肘", "zhǒu", "elbow strike"),
                    ("採", "cǎi", "yank"),
                    ("靠", "liè", "split"),
                    ("分脚", "fēn jiǎo", "side kick"),
                    ("蹬脚", "dēng jiǎo", "heel kick"),
                ],
            ),
            (
                "Doers",
                vec![
                    ("", "shīfu", "master"),
                    ("", "lǎoshī", "teacher"),
                    ("", "", "student"),
                ],
            ),
            (
                "Doings",
                vec![
                    ("起", "qǐ", "to raise"),
                    ("听", "tīng", "to listen"),
                    ("停", "tíng", "to stop"),
                    ("练习", "liànxí", "to practice"),
                    ("面对", "miàn duì", "to face"),
                    ("弓腿", "gōng tuǐ", "working leg, bow stand"), // bend your legs?
                    ("坐腿", "zuò tuǐ", "sitting on legs"),
                    ("转腰", "zhuǎn yāo", "turn your waist"),
                    ("转身", "zhuǎn shēn", "turn your body"),
                    ("松肩", "sōng jiān", "relax your shoulders"),
                    ("??", "sōng kua", "relax your hips/pelvis?"),
                    ("进步", "Jìnbù", "step forward"),
                    ("上步", "shàng bù", "step again"),
                    ("跟步", "gēn bù", "follow step"),
                    ("半步", "bàn bù", "half step"),
                    ("退步", "tuì bù", "step back"),
                    ("下课", "xià kè", "close class"),
                ],
            ),
            (
                "Animals",
                vec![
                    ("雀", "què", "bird"),
                    ("鹤", "hè", "crane"),
                    ("虎", "hǔ ", "tiger"),
                    ("猴", "hóu", "monkey"),
                    ("马", "mǎ", "horse"),
                    ("鸡", "jī", "rooster"),
                    ("蛇", "shé", "snake "),
                    ("鱼", "yú", "fish"),
                    ("龍", "lóng", "dragon"),
                    ("", "", "swallow"),
                    ("", "", "lion"),
                    ("", "", "wasp"),
                    ("", "", "ape"),
                    ("", "", "cat"),
                    ("", "", "mouse"),
                    ("", "", "insect"),
                ],
            ),
            (
                "Adjectives",
                vec![("白", "bái", "white"), ("大", "dà", "big")],
            ),
            (
                "Energies",
                vec![
                    ("", "zhan", ""),
                    ("", "nian", ""),
                    ("", "lian", ""),
                    ("", "sui", ""),
                    ("", "kang", ""),
                    ("", "diu", ""),
                    ("", "ding", ""),
                    ("", "bian", ""),
                    ("听劲", "tīng jìn", "listening energy"),
                    ("化劲", "huà jìn", "transforming energy"),
                    ("拿劲", "ná jìn", "controlling energy"),
                    ("发劲", "fā jìn", "releasing energy"),
                    ("发劲", "dong jìn", "releasing energy"),
                ],
            ),
            (
                "Sayings",
                vec![
                    ("很好", "Hěn hǎo", "very good"),
                    ("慢慢吃", "Màn man chī", "Enjoy your meal"),
                    ("", "zailaiiizi", "One more time"),
                ],
            ),
        ];
        let proverbs = vec![
            ("隨機應變", "Suíjīyìngbiàn", "Adapt to changing conditions"),
            ("熟能生巧", "Shúnéngshēngqiǎo", "Practice makes perfect"),
            (
                "欲速則不達",
                "Yù sù zé bù dá",
                "Hastiness stands in the way of success",
            ),
            (
                "有人若无人 无人若有人",
                "Yǒu rén ruò wú rén. Wú rén ruò yǒu rén.",
                "Somebody seems like nobody. Nobody seems like somebody.",
            ),
            (
                "你不动，我不动， 你欲动，我先动",
                "Nǐ bù dòng, wǒ bù dòng, nǐ yù dòng, wǒ xiān dòng",
                "You don't move, I don't move. You intend to move, I'll move first.",
            ),
        ];
        html! {
            div .names-of-set {
                @for (header, terms) in terms {
                    div .full {
                        h3 { (header) }
                    }
                    @for (characters, pinyin, translation) in terms {
                        div .characters { (characters) }
                        div .pinyin { (pinyin) }
                        div .english { (translation) }
                    }
                }
            }

            div .proverbs {
                div { h3 { "Proverbs" } }

                @for (characters, pinyin, translation) in proverbs {
                    div .characters { (characters) }
                    div .pinyin { (pinyin) }
                    div .english { (translation) }
                }
            }
        }
    } else {
        html! {}
    }
}
