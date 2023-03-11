#/bin/bash
export DATABASE_URL="postgres://open_tab@localhost/open_tab_v2"

dropdb open_tab_v2
createdb -O open_tab open_tab_v2
sea-orm-cli migrate up
sea-orm-cli generate entity -o open_tab_entities/src/schema/automatic --ignore-tables seaql_migrations,ballot,adjudicator_team_score,adjudicator_speech_score,ballot_speech,ballot_team,participant

echo "use super::manual::*;" >> "open_tab_entities/src/schema/automatic/mod.rs"