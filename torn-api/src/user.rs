use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::collections::{BTreeMap, HashMap};

use torn_api_macros::{ApiCategory, IntoOwned};

use crate::de_util;

pub use crate::common::{Attack, AttackFull, LastAction, Status};

#[derive(Debug, Clone, Copy, ApiCategory)]
#[api(category = "user")]
#[non_exhaustive]
pub enum UserSelection {
    #[api(type = "Basic", flatten)]
    Basic,
    #[api(type = "Profile", flatten)]
    Profile,
    #[api(type = "Discord", field = "discord")]
    Discord,
    #[api(type = "PersonalStats", field = "personalstats")]
    PersonalStats,
    #[api(type = "CriminalRecord", field = "criminalrecord")]
    Crimes,
    #[api(type = "BTreeMap<i32, Attack>", field = "attacks")]
    AttacksFull,
    #[api(type = "BTreeMap<i32, AttackFull>", field = "attacks")]
    Attacks,
    #[api(type = "HashMap<Icon, &str>", field = "icons")]
    Icons,
}

pub type Selection = UserSelection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Enby,
}

#[derive(Debug, IntoOwned)]
pub struct Faction<'a> {
    pub faction_id: i32,
    pub faction_name: &'a str,
    pub days_in_faction: i16,
    pub position: &'a str,
    pub faction_tag: Option<&'a str>,
}

fn deserialize_faction<'de, D>(deserializer: D) -> Result<Option<Faction<'de>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum Field {
        FactionId,
        FactionName,
        DaysInFaction,
        Position,
        FactionTag,
    }

    struct FactionVisitor;

    impl<'de> Visitor<'de> for FactionVisitor {
        type Value = Option<Faction<'de>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct Faction")
        }

        fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
        where
            V: MapAccess<'de>,
        {
            let mut faction_id = None;
            let mut faction_name = None;
            let mut days_in_faction = None;
            let mut position = None;
            let mut faction_tag = None;

            while let Some(key) = map.next_key()? {
                match key {
                    Field::FactionId => {
                        faction_id = Some(map.next_value()?);
                    }
                    Field::FactionName => {
                        faction_name = Some(map.next_value()?);
                    }
                    Field::DaysInFaction => {
                        days_in_faction = Some(map.next_value()?);
                    }
                    Field::Position => {
                        position = Some(map.next_value()?);
                    }
                    Field::FactionTag => {
                        faction_tag = map.next_value()?;
                    }
                }
            }
            let faction_id = faction_id.ok_or_else(|| de::Error::missing_field("faction_id"))?;
            let faction_name =
                faction_name.ok_or_else(|| de::Error::missing_field("faction_name"))?;
            let days_in_faction =
                days_in_faction.ok_or_else(|| de::Error::missing_field("days_in_faction"))?;
            let position = position.ok_or_else(|| de::Error::missing_field("position"))?;

            if faction_id == 0 {
                Ok(None)
            } else {
                Ok(Some(Faction {
                    faction_id,
                    faction_name,
                    days_in_faction,
                    position,
                    faction_tag,
                }))
            }
        }
    }

    const FIELDS: &[&str] = &[
        "faction_id",
        "faction_name",
        "days_in_faction",
        "position",
        "faction_tag",
    ];
    deserializer.deserialize_struct("Faction", FIELDS, FactionVisitor)
}

#[derive(Debug, IntoOwned, Deserialize)]
pub struct Basic<'a> {
    pub player_id: i32,
    pub name: &'a str,
    pub level: i16,
    pub gender: Gender,
    pub status: Status<'a>,
}

#[derive(Debug, Clone, IntoOwned, PartialEq, Eq, Deserialize)]
#[into_owned(identity)]
pub struct Discord {
    #[serde(
        rename = "userID",
        deserialize_with = "de_util::empty_string_int_option"
    )]
    pub user_id: Option<i32>,
    #[serde(rename = "discordID", deserialize_with = "de_util::string_is_long")]
    pub discord_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LifeBar {
    pub current: i16,
    pub maximum: i16,
    pub increment: i16,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EliminationTeam2022 {
    Firestarters,
    HardBoiled,
    QuackAddicts,
    RainMen,
    TotallyBoned,
    RawringThunder,
    DirtyCops,
    LaughingStock,
    JeanTherapy,
    #[serde(rename = "satants-soldiers")]
    SatansSoldiers,
    WolfPack,
    Sleepyheads,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EliminationTeam {
    Backstabbers,
    Cheese,
    DeathsDoor,
    RegularHumanPeople,
    FlowerRangers,
    ReligiousExtremists,
    Hivemind,
    CapsLockCrew,
}

#[derive(Debug, Clone, IntoOwned)]
#[into_owned(identity)]
pub enum Competition {
    Elimination {
        score: i32,
        attacks: i16,
        team: EliminationTeam,
    },
    DogTags {
        score: i32,
        position: Option<i32>,
    },
    Unknown,
}

fn deserialize_comp<'de, D>(deserializer: D) -> Result<Option<Competition>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    enum Field {
        Name,
        Score,
        Team,
        Attacks,
        TeamName,
        Position,
        #[serde(other)]
        Ignore,
    }

    #[derive(Deserialize)]
    enum CompetitionName {
        Elimination,
        #[serde(rename = "Dog Tags")]
        DogTags,
        #[serde(other)]
        Unknown,
    }

    struct CompetitionVisitor;

    impl<'de> Visitor<'de> for CompetitionVisitor {
        type Value = Option<Competition>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct Competition")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_map(self)
        }

        fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
        where
            V: MapAccess<'de>,
        {
            let mut team = None;
            let mut score = None;
            let mut attacks = None;
            let mut name = None;
            let mut position = None;

            while let Some(key) = map.next_key()? {
                match key {
                    Field::Name => {
                        name = Some(map.next_value()?);
                    }
                    Field::Score => {
                        score = Some(map.next_value()?);
                    }
                    Field::Attacks => {
                        attacks = Some(map.next_value()?);
                    }
                    Field::Position => {
                        position = Some(map.next_value()?);
                    }
                    Field::Team => {
                        let team_raw: &str = map.next_value()?;
                        team = if team_raw.is_empty() {
                            None
                        } else {
                            Some(match team_raw {
                                "backstabbers" => EliminationTeam::Backstabbers,
                                "cheese" => EliminationTeam::Cheese,
                                "deaths-door" => EliminationTeam::DeathsDoor,
                                "regular-human-people" => EliminationTeam::RegularHumanPeople,
                                "flower-rangers" => EliminationTeam::FlowerRangers,
                                "religious-extremists" => EliminationTeam::ReligiousExtremists,
                                "hivemind" => EliminationTeam::Hivemind,
                                "caps-lock-crew" => EliminationTeam::CapsLockCrew,
                                _ => Err(de::Error::unknown_variant(team_raw, &[]))?,
                            })
                        }
                    }
                    _ => (),
                }
            }

            let name = name.ok_or_else(|| de::Error::missing_field("name"))?;

            match name {
                CompetitionName::Elimination => {
                    if let Some(team) = team {
                        let score = score.ok_or_else(|| de::Error::missing_field("score"))?;
                        let attacks = attacks.ok_or_else(|| de::Error::missing_field("attacks"))?;
                        Ok(Some(Competition::Elimination {
                            team,
                            score,
                            attacks,
                        }))
                    } else {
                        Ok(None)
                    }
                }
                CompetitionName::DogTags => {
                    let score = score.ok_or_else(|| de::Error::missing_field("score"))?;
                    let position = position.ok_or_else(|| de::Error::missing_field("position"))?;

                    Ok(Some(Competition::DogTags { score, position }))
                }
                CompetitionName::Unknown => Ok(Some(Competition::Unknown)),
            }
        }
    }

    deserializer.deserialize_option(CompetitionVisitor)
}

#[derive(Debug, IntoOwned, Deserialize)]
pub struct Profile<'a> {
    pub player_id: i32,
    pub name: &'a str,
    pub rank: &'a str,
    pub level: i16,
    pub gender: Gender,
    pub age: i32,

    pub life: LifeBar,
    pub last_action: LastAction,
    #[serde(deserialize_with = "deserialize_faction")]
    pub faction: Option<Faction<'a>>,
    pub job: EmploymentStatus,
    pub status: Status<'a>,

    #[serde(deserialize_with = "deserialize_comp")]
    pub competition: Option<Competition>,

    #[serde(deserialize_with = "de_util::int_is_bool")]
    pub revivable: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PersonalStats {
    #[serde(rename = "attackswon")]
    pub attacks_won: i32,
    #[serde(rename = "attackslost")]
    pub attacks_lost: i32,
    #[serde(rename = "defendswon")]
    pub defends_won: i32,
    #[serde(rename = "defendslost")]
    pub defends_lost: i32,
    #[serde(rename = "statenhancersused")]
    pub stat_enhancers_used: i32,
    pub refills: i32,
    #[serde(rename = "drugsused")]
    pub drugs_used: i32,
    #[serde(rename = "xantaken")]
    pub xanax_taken: i32,
    #[serde(rename = "lsdtaken")]
    pub lsd_taken: i32,
    #[serde(rename = "networth")]
    pub net_worth: i64,
    #[serde(rename = "energydrinkused")]
    pub cans_used: i32,
    #[serde(rename = "boostersused")]
    pub boosters_used: i32,
    pub awards: i16,
    pub elo: i16,
    #[serde(rename = "daysbeendonator")]
    pub days_been_donator: i16,
    #[serde(rename = "bestdamage")]
    pub best_damage: i32,
}

#[derive(Deserialize)]
pub struct Crimes1 {
    pub selling_illegal_products: i32,
    pub theft: i32,
    pub auto_theft: i32,
    pub drug_deals: i32,
    pub computer_crimes: i32,
    pub murder: i32,
    pub fraud_crimes: i32,
    pub other: i32,
    pub total: i32,
}

#[derive(Deserialize)]
pub struct Crimes2 {
    pub vandalism: i32,
    pub theft: i32,
    pub counterfeiting: i32,
    pub fraud: i32,
    #[serde(rename = "illicitservices")]
    pub illicit_services: i32,
    #[serde(rename = "cybercrime")]
    pub cyber_crime: i32,
    pub extortion: i32,
    #[serde(rename = "illegalproduction")]
    pub illegal_production: i32,
    pub total: i32,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum CriminalRecord {
    Crimes1(Crimes1),
    Crimes2(Crimes2),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Icon(i16);

impl Icon {
    pub const SUBSCRIBER: Self = Self(4);
    pub const LEVEL_100: Self = Self(5);
    pub const GENDER_MALE: Self = Self(6);
    pub const GENDER_FEMALE: Self = Self(7);
    pub const MARITAL_STATUS: Self = Self(8);
    pub const FACTION_MEMBER: Self = Self(9);
    pub const PLAYER_COMMITTEE: Self = Self(10);
    pub const STAFF: Self = Self(11);

    pub const COMPANY: Self = Self(27);
    pub const BANK_INVESTMENT: Self = Self(29);
    pub const PROPERTY_VAULT: Self = Self(32);
    pub const DUKE_LOAN: Self = Self(33);

    pub const DRUG_COOLDOWN: Self = Self(53);

    pub const FEDDED: Self = Self(70);
    pub const TRAVELLING: Self = Self(71);
    pub const FACTION_LEADER: Self = Self(74);
    pub const TERRITORY_WAR: Self = Self(75);

    pub const FACTION_RECRUIT: Self = Self(81);
    pub const STOCK_MARKET: Self = Self(84);
}

impl<'de> Deserialize<'de> for Icon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IconVisitor;

        impl<'de> Visitor<'de> for IconVisitor {
            type Value = Icon;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "struct Icon")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Some(suffix) = v.strip_prefix("icon") {
                    Ok(Icon(suffix.parse().map_err(|_e| {
                        de::Error::invalid_value(de::Unexpected::Str(suffix), &"&str \"IconXX\"")
                    })?))
                } else {
                    Err(de::Error::invalid_value(
                        de::Unexpected::Str(v),
                        &"&str \"iconXX\"",
                    ))
                }
            }
        }

        deserializer.deserialize_str(IconVisitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum Job {
    Director,
    Employee,
    Education,
    Army,
    Law,
    Casino,
    Medical,
    Grocer,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Company {
    PlayerRun {
        name: String,
        id: i32,
        company_type: u8,
    },
    CityJob,
}

impl<'de> Deserialize<'de> for Company {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CompanyVisitor;

        impl<'de> Visitor<'de> for CompanyVisitor {
            type Value = Company;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("enum Company")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[allow(clippy::enum_variant_names)]
                #[derive(Deserialize)]
                #[serde(rename_all = "snake_case")]
                enum Field {
                    CompanyId,
                    CompanyName,
                    CompanyType,
                    #[serde(other)]
                    Other,
                }

                let mut id = None;
                let mut name = None;
                let mut company_type = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::CompanyId => {
                            id = Some(map.next_value()?);
                            if id == Some(0) {
                                return Ok(Company::CityJob);
                            }
                        }
                        Field::CompanyType => company_type = Some(map.next_value()?),
                        Field::CompanyName => {
                            name = Some(map.next_value()?);
                        }
                        Field::Other => (),
                    }
                }

                let id = id.ok_or_else(|| de::Error::missing_field("company_id"))?;
                let name = name.ok_or_else(|| de::Error::missing_field("company_name"))?;
                let company_type =
                    company_type.ok_or_else(|| de::Error::missing_field("company_type"))?;

                Ok(Company::PlayerRun {
                    name,
                    id,
                    company_type,
                })
            }
        }

        deserializer.deserialize_map(CompanyVisitor)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmploymentStatus {
    pub job: Job,
    #[serde(flatten)]
    pub company: Company,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{async_test, setup, Client, ClientTrait};

    #[async_test]
    async fn user() {
        let key = setup();

        let response = Client::default()
            .torn_api(key)
            .user(|b| {
                b.selections([
                    Selection::Basic,
                    Selection::Discord,
                    Selection::Profile,
                    Selection::PersonalStats,
                    Selection::Crimes,
                    Selection::Attacks,
                ])
            })
            .await
            .unwrap();

        response.basic().unwrap();
        response.discord().unwrap();
        response.profile().unwrap();
        response.personal_stats().unwrap();
        response.crimes().unwrap();
        response.attacks().unwrap();
        response.attacks_full().unwrap();
    }

    #[async_test]
    async fn not_in_faction() {
        let key = setup();

        let response = Client::default()
            .torn_api(key)
            .user(|b| b.id(28).selections([Selection::Profile]))
            .await
            .unwrap();

        let faction = response.profile().unwrap().faction;

        assert!(faction.is_none());
    }

    #[async_test]
    async fn bulk() {
        let key = setup();

        let response = Client::default()
            .torn_api(key)
            .users([1, 2111649, 374272176892674048i64], |b| {
                b.selections([Selection::Basic])
            })
            .await;

        response.get(&1).as_ref().unwrap().as_ref().unwrap();
        response.get(&2111649).as_ref().unwrap().as_ref().unwrap();
    }

    #[async_test]
    async fn discord() {
        let key = setup();

        let response = Client::default()
            .torn_api(key)
            .user(|b| b.id(374272176892674048i64).selections([Selection::Basic]))
            .await
            .unwrap();

        assert_eq!(response.basic().unwrap().player_id, 2111649);
    }

    #[async_test]
    async fn fedded() {
        let key = setup();

        let response = Client::default()
            .torn_api(key)
            .user(|b| b.id(1900654).selections([Selection::Icons]))
            .await
            .unwrap();

        let icons = response.icons().unwrap();

        assert!(icons.contains_key(&Icon::FEDDED))
    }
}
