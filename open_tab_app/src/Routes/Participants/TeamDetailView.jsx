import React, { useContext } from "react";
import { useState } from "react";
import { ErrorHandlingContext, executeAction } from "../../Action";
import { TournamentContext } from "../../TournamentContext";
import Button from "../../UI/Button";
import { useEffect } from "react";
import _ from "lodash";
import TextField from "../../UI/TextField";
import { RowBlockerContext } from "./RowBlocker";
import { useView } from "../../View";

export function TeamDetailView({ team, onChange }) {
    let [name, setName] = useState(team.name);
    let tournamentContext = useContext(TournamentContext);
    let errorContext = useContext(ErrorHandlingContext);

    let currentView = { type: "ParticipantsList", tournament_uuid: tournamentContext.uuid };
    let { teams } = useView(currentView, { "teams": {}, "adjudicators": {} });

    let teamNames = new Set(Object.values(teams).map(
        team => team.name
    ));

    useEffect(() => {
        setName(team.name);
    }, [team.uuid]);

    let hasChanges = !_.eq(
        team.name,
        name
    );

    let { block } = useContext(RowBlockerContext);

    useEffect(() => {
            if (hasChanges) {
                let lease = block();
                return () => {
                    lease.unblock();
                }
            }
        },
        [hasChanges]
    );

    return <div className="w-full">
        <label className="font-bold">Team Name</label>
        <div className="mb-2">
            <TextField value={name} onChange={(e) => {
                setName(e.target.value);
            }} />
        </div>

        {hasChanges && <Button onClick={async () => {
            if (teamNames.has(name)) {
                if (!await confirm("A team with this name already exists. Are you sure to rename this team? If you need to change the team of this speaker, use the 'Move to Team' option above instead.")) {
                    return;
                }
            }
            executeAction("UpdateTeams", {
                tournament_id: tournamentContext.uuid, updates: [
                    {
                        uuid: team.uuid,
                        name: name
                    }
                ]
            }, errorContext.handleError);
        }}> Rename Team </Button>}

    </div>;
}
