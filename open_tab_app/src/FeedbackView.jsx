import { Outlet } from "react-router"
import { TournamentContext } from "./TournamentContext";
import { useContext } from 'react';
import { useView } from "./View";

function get_cell_value(value) {
    switch (value.type) {
        case "Average":
            return value.avg.toFixed(2);
        case "Percentage":
            return (value.percentage * 100).toFixed(2) + "%";
        default:
            return "-"
    }
}

export function FeedbackOverviewTable() {
    let context = useContext(TournamentContext);
    let feedback_overview = useView({type: "FeedbackOverview", tournament_uuid: context.uuid}, null);
    console.log(feedback_overview);

    if (feedback_overview == null) {
        return <div>Loading...</div>
    }

    return <div>
        <div>
        <table>
            <thead>
                <tr>
                    <th>Participant</th>
                    {feedback_overview.summary_columns.map((column, idx) => <th key={idx}>{column.title}</th>)}
                </tr>
            </thead>
            <tbody>
                {feedback_overview.participant_entries.map((participant, idx) => <tr key={idx}>
                    <td className="text-right">{participant.participant_name}</td>
                    {feedback_overview.summary_columns.map((column, idx) => <td key={idx} className="text-center">{
                        get_cell_value(participant.score_summaries[column.question_id])
                    }</td>)}
                </tr>)}
            </tbody>
        </table>
        </div>
        <Outlet />
    </div>
}


export function FeedbackDetailViewRoute(props) {
    return <p>Detail</p>
}

export function FeedbackOverviewRoute(props) {
    return <div><FeedbackOverviewTable /></div>
}