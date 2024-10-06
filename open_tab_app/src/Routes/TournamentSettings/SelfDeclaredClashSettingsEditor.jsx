import React from "react";
import { executeAction } from "../../Action";
import { TournamentContext } from "../../TournamentContext";

export default function SelfDeclaredClashSettingsEditor({statusView}) {
    let tournament = React.useContext(TournamentContext);
    return <div>
        <h1 className="font-bold">Clash Declaration</h1>
        <div className="flex space-x-4">
            <label className="flex items-center space-x-2">
                <input
                    type="radio"
                    value="y"
                    checked={!statusView.allow_self_declared_clashes}
                    onChange={() => {
                        executeAction("UpdateTournament", {
                            tournament_id: tournament.uuid,
                            allow_self_declared_clashes: false,
                            allow_speaker_self_declared_clashes: false
                        });
                    }}
                    className="form-radio"
                />
                <span>Participants can not self-declare clashes</span>
            </label>
            <label className="flex items-center space-x-2">
                <input
                    type="radio"
                    value="n"
                    checked={statusView.allow_self_declared_clashes && !statusView.allow_speaker_self_declared_clashes}
                    onChange={() => {
                        executeAction("UpdateTournament", {
                            tournament_id: tournament.uuid,
                            allow_self_declared_clashes: true,
                            allow_speaker_self_declared_clashes: false
                        });
                    }}
                    className="form-radio"
                />
                <span>Only adjudicators can self-declare clashes</span>
            </label>
            <label className="flex items-center space-x-2">
                <input
                    type="radio"
                    value="n"
                    checked={statusView.allow_self_declared_clashes && statusView.allow_speaker_self_declared_clashes}
                    onChange={() => {
                        executeAction("UpdateTournament", {
                            tournament_id: tournament.uuid,
                            allow_self_declared_clashes: true,
                            allow_speaker_self_declared_clashes: true
                        });
                    }}
                    className="form-radio"
                />
                <span>Everyone can self-declare clashes</span>
            </label>
        </div>
    </div>
}