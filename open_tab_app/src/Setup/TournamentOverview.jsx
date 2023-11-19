import React, { useEffect, useState } from 'react';

import { invoke } from '@tauri-apps/api/tauri';
import TournamentCreationForm from './TournamentCreationForm';

const TournamentOverview = ({ tournaments, onCreateNew }) => {
  const [selectedTournament, setSelectedTournament] = useState(null);

  const handleRowClick = (uuid) => {
    setSelectedTournament(uuid);
  };

  const handleRowDoubleClick = (uuid) => {
    invoke("open_tournament", { tournamentId: uuid })
  };

  const handleNewTournament = () => {
    // Placeholder for new tournament action
    onCreateNew();
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
    let [isCreatingNew, setIsCreatingNew] = useState(false);

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

      {
        isCreatingNew ?
        <TournamentCreationForm onAbort={() => {
          setIsCreatingNew(false);
        }
        } onSubmit={(config) => {
          invoke("create_tournament", {config}).then((tournament) => {
            setIsCreatingNew(false);
            invoke("open_tournament", { tournamentId: tournament.uuid })
          });
        }} />
        :
        <TournamentOverview tournaments={tournaments} onCreateNew={() => {
          setIsCreatingNew(true);
        }} />
      }
    </div>;
}

export default TournamentManager;
