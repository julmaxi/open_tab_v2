import { useContext, useState } from "react";
import { TournamentContext } from "../../TournamentContext";
import { useView } from "../../View";
import { useSettings } from "../../settings";

import ModalOverlay from "../../UI/Modal";

import RemoteSelector from "./RemoteSelector";
import { invoke } from "@tauri-apps/api/tauri";
import SettingsEditor from "./SettingsEditor";
import { executeAction } from "../../Action";
import SelfDeclaredClashSettingsEditor from "./SelfDeclaredClashSettingsEditor";

export default function TournamentViewRoute(props) {
    let tournament = useContext(TournamentContext);

    let statusView = useView({ type: "TournamentStatus", tournament_uuid: tournament.uuid }, null);
    let settings = useSettings();
    let tournamentId = useContext(TournamentContext).uuid;

    console.log(statusView);

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
                            <SelfDeclaredClashSettingsEditor statusView={statusView} />
                            :
                            []
                    }

                </div>
                :
                <p>Loading</p>
        }
    </div>;
}