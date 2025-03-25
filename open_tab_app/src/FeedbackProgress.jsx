import { useView } from "./View";
import { TournamentContext } from "./TournamentContext";
import { ErrorHandlingContext } from "./Action.js";
import { useContext } from 'react';
import { TabGroup, Tab } from "./TabGroup.jsx";
import { SortableTable } from "./SortableTable";

export function FeedbackProgressTable({rows, rounds, isTeams}) {
    let data = [];

    for (let row of rows) {
        let out = {
            name: row.name,
            uuid: row.team_id || row.participant_id,
        };
        let total = 0;
        for (let round of rounds) {
            let progress = row.round_progress[round.uuid];

            if (progress == undefined) {
                out[round.uuid] = 0;
                continue;
            }
            else {
                let missing = progress.submission_requirement - progress.submission_count;
                total += missing;
                out[round.uuid] = missing;
            }
        }
        out.total = total;
        data.push(out);
    }

    let highlightedCellFactors = (value, rowIdx, colIdx, row) => {
        return <td key={colIdx} className={(value > 0 ? "text-red-600" : "text-green-600") + ""}>{value}</td>
    }
        

    let columns = [{
        header: "Adjudicator",
        key: "name",
    }, {
        header: "Tot.",
        key: "total",
        cellFactory: highlightedCellFactors,
    }, ...rounds.map(
        (round) => ({
            header: round.name,
            key: round.uuid,
            cellFactory: highlightedCellFactors,
        })
    )];

    return <SortableTable rowId={"uuid"} columns={columns} data={data} selectedRowId={null} />
}

export function FeedbackProgressRoute() {
    let tournamentId = useContext(TournamentContext).uuid;
    let errorContext = useContext(ErrorHandlingContext);

    let progress = useView({type: "FeedbackProgress", tournament_uuid: tournamentId}, null);
    
    return <div className="flex align-middle justify-center w-full h-full flex-col">
        <TabGroup>
            <Tab name="Adjudicators">
                <div>
                    {progress !== null ? <FeedbackProgressTable rows={progress.adjudicator_feedback_info} rounds={progress.rounds} /> : "Loading..."}
                </div>
            </Tab>
            <Tab name="Teams">
                <div>
                    {progress !== null ? <FeedbackProgressTable rows={progress.team_feedback_info} rounds={progress.rounds} /> : "Loading..."}
                </div>
            </Tab>
        </TabGroup>
    </div>
}