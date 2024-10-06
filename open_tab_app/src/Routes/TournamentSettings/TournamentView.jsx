import { useContext, useState } from "react";
import { TournamentContext } from "../../TournamentContext";
import { useView } from "../../View";
import { useSettings } from "../../settings";

import ModalOverlay from "../../UI/Modal";

import RemoteSelector from "./RemoteSelector";
import { invoke } from "@tauri-apps/api/tauri";
import SettingsEditor from "./SettingsEditor";
import { executeAction } from "../../Action";

export default function TournamentViewRoute(props) {
    let tournament = useContext(TournamentContext);

    let statusView = useView({ type: "TournamentStatus", tournament_uuid: tournament.uuid }, null);
    let settings = useSettings();
    let tournamentId = useContext(TournamentContext).uuid;

    return <div className="h-full w-full p-2">
        <h1 className="font-bold">Remote Settings</h1>
        {
            statusView ?
                <div className="h-full w-full flex flex-col">
                    <RemoteSelector
                        knownRemotes={settings.known_remotes || []}
                        currentRemoteUrl={statusView.remote_url}
                        onSetRemote={(url) => {
                            invoke("set_remote", { remoteUrl: url, tournamentId: tournamentId });
                        }
                        }
                    />
                    <p>
                        {statusView.annoucements_password}
                    </p>

                    <div className="pt-2">
                        {statusView.remote_url !== null ? <SettingsEditor /> : []}
                    </div>

                    {
                        statusView.remote_url ?
                            <div>
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
                                                    allow_self_declared_clashes: false
                                                });
                                            }}
                                            className="form-radio"
                                        />
                                        <span>Users can not self-declare clashes</span>
                                    </label>
                                    <label className="flex items-center space-x-2">
                                        <input
                                            type="radio"
                                            value="n"
                                            checked={statusView.allow_self_declared_clashes}
                                            onChange={() => {
                                                executeAction("UpdateTournament", {
                                                    tournament_id: tournament.uuid,
                                                    allow_self_declared_clashes: true
                                                });
                                            }}
                                            className="form-radio"
                                        />
                                        <span>Users can self-declare clashes</span>
                                    </label>
                                </div>
                            </div>
                            : []
                    }


                </div>
                :
                <p>Loading</p>
        }
    </div>;
}