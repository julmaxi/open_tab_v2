import React, { useContext } from "react";
import { useState } from "react";
import { ErrorHandlingContext, executeAction } from "../../Action";
import { TournamentContext } from "../../TournamentContext";
import Button from "../../UI/Button";
import { useEffect } from "react";
import _ from "lodash";
import TextField from "../../UI/TextField";

export function TeamDetailView({ team, onChange }) {
    let [name, setName] = useState(team.name);
    let tournamentContext = useContext(TournamentContext);
    let errorContext = useContext(ErrorHandlingContext);

    useEffect(() => {
        setName(team.name);
    }, [team.uuid]);

    let hasChanges = !_.eq(
        team.name,
        name
    );

    return <div className="w-full">
        <label>Team Name</label>
        <div className="mb-2">
            <TextField value={name} onChange={(e) => {
                setName(e.target.value);
            }} />
        </div>

        {hasChanges && <Button onClick={() => {
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
