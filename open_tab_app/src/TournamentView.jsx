import { useContext } from "react";
import { TournamentContext } from "./TournamentContext";
import { useView } from "./View";


export default function TournamentViewRoute(props) {
    let tournament = useContext(TournamentContext);

    let statusView = useView({type: "TournamentStatus", tournament_uuid: tournament.uuid}, null);

    return <div className="h-full w-full flex flex-col">
        {
            statusView ? <p>
                {statusView.annoucements_password}
            </p>
            : <p>Loading</p>
        }
    </div>  
}