import React, { useContext } from "react";
import { useView } from "../../View";
import { SortableTable } from "../../SortableTable";
import { TournamentContext } from "@/TournamentContext";

function InstitutionsListView() {
    const tournament_uuid = useContext(TournamentContext).uuid;
    const institutionsView = useView({ type: "Institutions", include_statistics: true, tournament_uuid }, { institutions: [] });

    console.log(institutionsView.institutions)
    const data = institutionsView.institutions.map((institution) => ({
        uuid: institution.uuid,
        name: institution.name,
        officialIdentifier: institution.official_identifier,
        numSpeakers: institution.statistics?.num_speakers || 0,
        numAdjudicators: institution.statistics?.num_adjudicators || 0,
    }));

    const columns = [
        { key: "name", header: "Institution Name" },
        { key: "officialIdentifier", header: "Identifier" },
        { key: "numSpeakers", header: "#Spk." },
        { key: "numAdjudicators", header: "#Adj." },
    ];

    return (
        <div className="w-full h-full">
            <SortableTable
                data={data}
                columns={columns}
                rowId="uuid"
                emptyText="No institutions available."
            />
        </div>
    );
}

export default InstitutionsListView;