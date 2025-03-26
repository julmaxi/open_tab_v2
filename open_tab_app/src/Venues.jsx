import { useContext } from "react";
import { useView } from "./View"
import { TournamentContext } from "./TournamentContext";
import { SortableTable, EditableCell } from "./SortableTable";
import { executeAction } from "./Action";
import Button from "./UI/Button";



export default function VenueOverview() {
    let tournamentId = useContext(TournamentContext).uuid;
    let venues = useView({type: "Venues", tournament_uuid: tournamentId}, {"venues": []});
    return <div className="h-full w-full flex flex-col">
        <div className="w-full flex-1 min-h-0">
            <SortableTable
                rowId={"uuid"}
                data={
                    venues.venues
                }
                columns={
                    [
                        {
                            "key": "ordering_index",
                            "header": "#",
                            cellFactory: (value, rowIdx, colIdx, rowValue) => {
                                return <EditableCell key={colIdx} value={value} onChange={
                                    (newIndex) => {
                                        let newVenue = {... rowValue};
                                        newVenue.ordering_index = parseInt(newIndex);
                                        executeAction("UpdateVenues", {updated_venues: [newVenue], tournament_id: tournamentId})
                                    }
                                } />
                            }      
                        },
                        {
                            "key": "name",
                            "header": "Name",
                            cellFactory: (value, rowIdx, colIdx, rowValue) => {
                                return <EditableCell key={colIdx} value={value} onChange={
                                    (newName) => {
                                        let newVenue = {... rowValue};
                                        newVenue.name = newName;
                                        executeAction("UpdateVenues", {updated_venues: [newVenue], tournament_id: tournamentId})
                                    }
                                } />
                            }
                        },
                    ]
                }
            />
        </div>
        <div className="flex-none w-full h-12 bg-gray-200">
            <button onClick={() => {
                executeAction("UpdateVenues", {tournament_id: tournamentId, added_venues: [{uuid: "00000000-0000-0000-0000-000000000000", name: "New Venue", ordering_index: venues.venues.length}]})

            }} className="h-full">Add Venue</button>
        </div>
    </div>
}