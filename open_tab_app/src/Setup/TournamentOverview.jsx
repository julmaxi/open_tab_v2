import React, { useEffect, useState } from 'react';

import { invoke } from '@tauri-apps/api/tauri';

const TournamentOverview = ({ tournaments }) => {
  const [selectedTournament, setSelectedTournament] = useState(null);

  const handleRowClick = (uuid) => {
    setSelectedTournament(uuid);
  };

  const handleRowDoubleClick = (uuid) => {
    // Placeholder for opening the tournament
    invoke("open_tournament", { tournamentId: uuid })
  };

  const handleNewTournament = () => {
    // Placeholder for new tournament action
    console.log("Creating a new tournament");
  };

  return (
    <div className="flex flex-col items-center justify-center h-screen w-full">
      <div className="overflow-y-auto shadow-md rounded-md">
        <table className="w-full bg-white">
          <thead className='sticky top-0'>
            <tr>
              <th className="bg-gray-200 text-left px-8 py-2">Tournament Name</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {tournaments.map((tournament, index) => (
              <tr
                key={tournament.uuid}
                onClick={() => handleRowClick(tournament.uuid)}
                onDoubleClick={() => handleRowDoubleClick(tournament.uuid)}
                className={`cursor-pointer ${selectedTournament === tournament.uuid ? 'bg-blue-300' : (index % 2 === 0 ? 'bg-gray-100' : 'bg-white')}`}
              >
                <td className="border-t px-8 py-2 text-center">{tournament.name}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <button
        onClick={handleNewTournament}
        className="mt-4 bg-blue-500 text-white py-2 px-6 rounded-full hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:ring-opacity-50"
      >
        New Tournament
      </button>
    </div>
  );
};


const TournamentManager = () => {
    let [tournaments, setTournaments] = useState([]);

    useEffect(() => {
        const run = async () => {
            await invoke("get_tournament_list", {
              }).then((tournaments) => {
                setTournaments(tournaments);
              });
        }
        run();
    }, []);

    return <div className="flex h-screen">
      <TournamentOverview tournaments={tournaments} />
    </div>;
}

export default TournamentManager;
