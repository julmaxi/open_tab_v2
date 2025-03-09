import React, { useState, useEffect } from 'react';
import Button from './UI/Button';
import { min } from 'lodash';

export const QUESTION_TYPES = {
    range: {
        displayName: "Range",
        renderer: (question, onUpdate) => <RangeQuestionEditor question={question} onUpdate={onUpdate} />,
        makeNewConfig: () => ({
            min: 0,
            max: 10,
            orientation: 'high',
            labels: {
                0: 'Low',
                10: 'High'
            }
        })
    },
    text: {
        displayName: "Text",
        renderer: (question, onUpdate) => <TextQuestionEditor question={question} onUpdate={onUpdate} />,
        makeNewConfig: () => ({})
    },
    yes_no: {
        displayName: "Yes/No",
        renderer: (question, onUpdate) => <YesNoQuestionEditor question={question} onUpdate={onUpdate} />,
        makeNewConfig: () => ({})
    }
}

const ChevronDown = () => (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" className="size-3 inline-block"><path d="M233.4 406.6c12.5 12.5 32.8 12.5 45.3 0l192-192c12.5-12.5 12.5-32.8 0-45.3s-32.8-12.5-45.3 0L256 338.7 86.6 169.4c-12.5-12.5-32.8-12.5-45.3 0s-12.5 32.8 0 45.3l192 192z"/></svg>
);

const ChevronRight = () => (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 512" className="size-3 inline-block"><path d="M310.6 233.4c12.5 12.5 12.5 32.8 0 45.3l-192 192c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L242.7 256 73.4 86.6c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l192 192z"/></svg>
);

// Main QuestionEditor component
const QuestionEditor = ({ question, onUpdate, onRemove }) => {
  let [showDetails, setShowDetails] = useState(false);
  const handleSharedPropertyChange = (property, value) => {
      const updated = { ...question, [property]: value };
      onUpdate(updated);
  };

  const handleSpecificPropertyChange = (property, value) => {
      const updated = { ...question, [property]: value };
      onUpdate(updated);
  };

  // Render specific editor based on question type
  const renderTypeSpecificEditor = () => {
    const type = question.type;
    const editor = QUESTION_TYPES[type];
    if (!editor) {
      return <p className="text-red-500">Error: Unknown question type {type}</p>;
    }

    return editor.renderer(question, handleSpecificPropertyChange);
  };

  return (
    <div className="relative space-y-2">
      <button
        className="absolute top-0 right-2 text-red-500"
        onClick={onRemove}
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4">
          <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>

      <div>
        <label className="block text-sm font-medium">Short Name</label>
        <input
          type="text"
          className="w-full p-2 border rounded"
          value={question.short_name || ''}
          onChange={(e) => handleSharedPropertyChange('short_name', e.target.value)}
        />
      </div>

      <button onClick={
            () => setShowDetails(!showDetails)
      }>{
            !showDetails ? <ChevronRight /> : <ChevronDown />
      } Details</button>

      <div className={showDetails ? 'block' : 'hidden'}>
        <SharedPropertiesEditor 
            question={question} 
            onChange={handleSharedPropertyChange} 
        />
        
        {renderTypeSpecificEditor()}
      </div>
    </div>
  );
};

// Component for editing properties shared by all question types
const SharedPropertiesEditor = ({ question, onChange }) => {
  return (
    <div className="space-y-4">      
      <p className='text-sm font-medium'>{QUESTION_TYPES[question.type].displayName}</p>
      <div>
        <label className="block text-sm font-medium mb-1">Full Name (Question Text)</label>
        <input
          type="text"
          className="w-full p-2 border rounded"
          value={question.full_name || ''}
          onChange={(e) => onChange('full_name', e.target.value)}
        />
      </div>

      <div>
        <label className="block text-sm font-medium mb-1">Description</label>
        <textarea
          className="w-full p-2 border rounded"
          value={question.description || ''}
          onChange={(e) => onChange('description', e.target.value)}
          rows={3}
        />
      </div>
      
      <div className="flex space-x-4">
        <div className="flex items-center">
          <input
            type="checkbox"
            id="is_required"
            className="mr-2"
            checked={question.is_required || false}
            onChange={(e) => onChange('is_required', e.target.checked)}
          />
          <label htmlFor="is_required" className="text-sm">Required</label>
        </div>
        
        <div className="flex items-center">
          <input
            type="checkbox"
            id="is_confidential"
            className="mr-2"
            checked={question.is_confidential || false}
            onChange={(e) => onChange('is_confidential', e.target.checked)}
          />
          <label htmlFor="is_confidential" className="text-sm">Confidential</label>
        </div>
      </div>
    </div>
  );
};

// Component for editing range-specific properties
const RangeQuestionEditor = ({ question, onChange }) => {
  const [minLabel, setMinLabel] = useState(question.labels ? question.labels[question.min] : '');
  const [maxLabel, setMaxLabel] = useState(question.labels ? question.labels[question.max] : '');

  const updateLabels = (min, max) => {
    const newLabels = { ...question.labels };
    
    // Delete old min/max keys if they exist
    if (question.min in newLabels) delete newLabels[question.min];
    if (question.max in newLabels) delete newLabels[question.max];
    
    // Add new min/max keys
    newLabels[min] = minLabel;
    newLabels[max] = maxLabel;
    
    onChange('labels', newLabels);
  };

  const handleMinChange = (value) => {
    onChange('min', parseInt(value, 10));
    updateLabels(parseInt(value, 10), question.max);
  };

  const handleMaxChange = (value) => {
    onChange('max', parseInt(value, 10));
    updateLabels(question.min, parseInt(value, 10));
  };

  const handleMinLabelChange = (value) => {
    setMinLabel(value);
    const newLabels = { ...question.labels, [question.min]: value };
    onChange('labels', newLabels);
  };

  const handleMaxLabelChange = (value) => {
    setMaxLabel(value);
    const newLabels = { ...question.labels, [question.max]: value };
    onChange('labels', newLabels);
  };

  return (
    <div className="space-y-4">
      <h3 className="text-md font-medium">Range Settings</h3>
      
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium mb-1">Minimum Value</label>
          <input
            type="number"
            className="w-full p-2 border rounded"
            value={question.min}
            onChange={(e) => handleMinChange(e.target.value)}
          />
        </div>
        
        <div>
          <label className="block text-sm font-medium mb-1">Maximum Value</label>
          <input
            type="number"
            className="w-full p-2 border rounded"
            value={question.max}
            onChange={(e) => handleMaxChange(e.target.value)}
          />
        </div>
        
        <div>
          <label className="block text-sm font-medium mb-1">Min Label</label>
          <input
            type="text"
            className="w-full p-2 border rounded"
            value={minLabel}
            onChange={(e) => handleMinLabelChange(e.target.value)}
          />
        </div>
        
        <div>
          <label className="block text-sm font-medium mb-1">Max Label</label>
          <input
            type="text"
            className="w-full p-2 border rounded"
            value={maxLabel}
            onChange={(e) => handleMaxLabelChange(e.target.value)}
          />
        </div>
      </div>
      
      <div>
        <label className="block text-sm font-medium mb-1">Orientation</label>
        <select
          className="w-full p-2 border rounded"
          value={question.orientation || 'high'}
          onChange={(e) => onChange('orientation', e.target.value)}
        >
          <option value="high">High (higher is better)</option>
          <option value="low">Low (lower is better)</option>
          <option value="mean">Mean (middle is ideal)</option>
        </select>
      </div>
    </div>
  );
};

// Component for editing text-specific properties
const TextQuestionEditor = ({ question, onChange }) => {
  // Currently text questions don't have specific properties
  // beyond the shared ones, but this component provides a place
  // to add them in the future
  return (
    <div className="space-y-4">
      <h3 className="text-md font-medium">Text Question Settings</h3>
      <p className="text-sm text-gray-500">No additional settings for text questions</p>
    </div>
  );
};

// Component for editing yes/no-specific properties
const YesNoQuestionEditor = ({ question, onChange }) => {
  // Currently yes/no questions don't have specific properties
  // beyond the shared ones, but this component provides a place
  // to add them in the future
  return (
    <div className="space-y-4">
      <h3 className="text-md font-medium">Yes/No Question Settings</h3>
      <p className="text-sm text-gray-500">No additional settings for yes/no questions</p>
    </div>
  );
};

export default QuestionEditor;