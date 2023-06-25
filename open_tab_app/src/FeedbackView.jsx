import { Outlet, useNavigate, useParams } from "react-router"
import { TournamentContext } from "./TournamentContext";
import { useContext, useState } from 'react';
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
    const navigate = useNavigate()


    let [selectedParticipant, setSelectedParticipant] = useState(null);

    if (feedback_overview == null) {
        return <div>Loading...</div>
    }

    return <div className="grid grid-cols-2">
        <div className="w-full h-screen overflow-scroll">
        <table>
            <thead>
                <tr>
                    <th>Participant</th>
                    {feedback_overview.summary_columns.map((column, idx) => <th key={idx}>{column.title}</th>)}
                </tr>
            </thead>
            <tbody>
                {feedback_overview.participant_entries.map((participant, idx) => <tr key={idx} className={
                    selectedParticipant == participant.participant_id ? "bg-blue-200" : ""
                } onClick={() => {
                    navigate("/feedback/" + participant.participant_id)
                    setSelectedParticipant(participant.participant_id);

                }}>
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

export function FeedbackOverviewRoute(props) {
    return <div><FeedbackOverviewTable /></div>
}

function FeedbackResponseDetails(props) {
    let { response } = props;
    let table_values = response.values.filter((value) => value.value.type != "String");
    let string_values = response.values.filter((value) => value.value.type == "String");

    return <div className="w-full border-b border-gray-200">
        <table>
            <thead>
                <tr>
                    {table_values.map(
                        (value, idx) => <th key={idx}>{value.question_short_name}</th>
                    )}
                </tr>
            </thead>
            <tbody>
                <tr>
                    {table_values.map(
                        (value, idx) => <td key={idx}>{value.value.val}</td>
                    )}
                </tr>
            </tbody>
        </table>
        
        {
            string_values.map((value, idx) => <div key={idx}>
                <h2>{value.question_short_name}</h2>
                <p>{value.value.val}</p>
            </div>)
        }
    </div>
}


export function FeedbackDetailViewRoute(props) {
    let { participantId } = useParams();
    let responses = useView({type: "FeedbackDetail", participant_id: participantId}, null);

    if (responses == null) {
        return <div>Loading...</div>
    }

    console.log(responses);

    return <div className="w-full h-screen overflow-scroll">
        {responses.responses.map((response, idx) => <FeedbackResponseDetails response={response} key={idx} />)}
    </div>

}