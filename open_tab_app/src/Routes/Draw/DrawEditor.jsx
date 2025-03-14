import { useParams } from "react-router-dom";
import DrawEditor from "./Draw";

export default function DrawEditorRoute() {
    let { roundId } = useParams();
    return <DrawEditor round_uuid={roundId} />;
}
