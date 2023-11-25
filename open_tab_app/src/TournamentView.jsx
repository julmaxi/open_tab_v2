import { useContext, useState } from "react";
import { TournamentContext } from "./TournamentContext";
import { useView } from "./View";
import { useSettings } from "./settings";

import ModalOverlay from "./Modal";

import RemoteSelector from "./RemoteSelector";
import { invoke } from "@tauri-apps/api/tauri";

export default function TournamentViewRoute(props) {
    let tournament = useContext(TournamentContext);

    let statusView = useView({type: "TournamentStatus", tournament_uuid: tournament.uuid}, null);
    let settings = useSettings();
    let tournamentId = useContext(TournamentContext).uuid;

    return <div className="h-full w-full">
        {
            statusView ? 
            <div className="h-full w-full flex flex-col">
                <RemoteSelector
                    knownRemotes={settings.known_remotes || []}
                    currentRemoteUrl={statusView.remote_url}
                    onSetRemote={(url) => {
                        invoke("set_remote", {remoteUrl: url, tournamentId: tournamentId});
                        }
                    }
                />
                <p>
                    {statusView.annoucements_password}
                </p>
            </div>  
            : 
            <p>Loading</p>
        }
    </div>;
}