import React, { useState, useMemo } from 'react';
import ButtonField from '../ButtonField';

import {open} from '@tauri-apps/api/dialog';

const TournamentCreationForm = ({ onAbort, onSubmit }) => {
  const [name, setName] = useState('');
  const [numberOfRounds, setNumberOfRounds] = useState(3);
  const [numberOfBreakRounds, setNumberOfBreakRounds] = useState(1);
  const [useDefaultFeedbackSystem, setUseDefaultFeedbackSystem] = useState(true);

  const roundName = useMemo(() => {
    switch (numberOfBreakRounds) {
      case 1: return 'Final';
      case 2: return 'Semi-Finals';
      case 3: return 'Quarter-Finals';
      case 4: return 'Octo-Finals';
      case 5: return 'Double-Octo-Finals';
      // Add more cases as needed
      default: return '1/' + Math.pow(2, numberOfBreakRounds - 1) + ' Finals';
    }
  }, [numberOfBreakRounds]);

  const handleSubmit = (e) => {
    e.preventDefault();
    onSubmit({ name, num_preliminaries: numberOfRounds, num_break_rounds: numberOfBreakRounds, use_default_feedback_system: useDefaultFeedbackSystem });
  };

  return (
    <div className="flex justify-center items-center h-screen w-screen">
      <form onSubmit={handleSubmit} className="space-y-4 bg-white p-6 rounded-lg shadow">
        <div>
          <label htmlFor="name" className="block text-sm font-medium text-gray-700">
            Name
          </label>
          <input
            type="text"
            id="name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
          />
        </div>

        <div>
          <label htmlFor="rounds" className="block text-sm font-medium text-gray-700">
            Number of Preliminaries
          </label>
          <input
            type="number"
            id="rounds"
            value={numberOfRounds}
            min={3}
            step={1}
            onChange={(e) => setNumberOfRounds(parseInt(e.target.value))}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
          />
        </div>

        <div>
          <label htmlFor="breakRounds" className="block text-sm font-medium text-gray-700">
            Number of Break Rounds
          </label>
          <input
            type="number"
            id="breakRounds"
            min={1}
            value={numberOfBreakRounds}
            onChange={(e) => setNumberOfBreakRounds(parseInt(e.target.value))}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
          />
          {roundName && (
            <p className="mt-2 text-sm text-gray-600">{roundName}</p>
          )}
        </div>

        <div className="mt-1">
            <div className="flex items-center">
              <input
                id="feedbackSystem"
                name="feedbackSystem"
                type="checkbox"
                checked={useDefaultFeedbackSystem}
                onChange={(e) => setUseDefaultFeedbackSystem(e.target.checked)}
                className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
              />
              <label htmlFor="feedbackSystem" className="ml-3 block text-sm font-medium text-gray-700">
                Use Default Feedback System
              </label>
            </div>
        </div>

        <div className="flex justify-end space-x-4">
          <button
            type="button"
            onClick={onAbort}
            className="inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500"
            role="secondary"
          >
            Cancel
          </button>
          <button
            type="submit"
            className="inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            role="primary"
          >
            Create
          </button>
        </div>
      </form>
    </div>
  );
};

export default TournamentCreationForm;