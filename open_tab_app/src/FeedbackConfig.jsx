import { useContext } from "react"
import { executeAction } from "./Action"
import Button from "./UI/Button"
import { open } from "@tauri-apps/api/dialog";

import { TournamentContext } from "./TournamentContext";
import { ErrorHandlingContext } from "./Action";

export function FeedbackConfigRoute() {
    let tournamentId = useContext(TournamentContext).uuid;
    let errorContext = useContext(ErrorHandlingContext);
    return <div className="flex align-middle justify-center w-full h-full flex-col">
        <Button role="primary" onClick={
            () => {
                open({directory: false}).then((result) => {
                    if (result !== undefined) {
                        executeAction("ImportFeedbackSystem", {
                            tournament_uuid: tournamentId,
                            template_path: result
                        }, errorContext.handleError)
                    }
                })
            }
        }>Import Feedback Template</Button>
        <p>This will replace the current feedback system.</p>
    </div>
}