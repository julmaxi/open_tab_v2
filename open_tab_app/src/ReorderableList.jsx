import React, { useState } from 'react';
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

// Item component with drag handle in the top-left
const SortableItem = ({ id, children }) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
  } = useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    position: 'relative',
    borderBottom: '1px solid #e2e8f0',
    padding: '16px',
    backgroundColor: 'white',
  };

  return (
    <div ref={setNodeRef} style={style}>
      {/* Drag handle in the top-left corner */}
      <div
        {...attributes}
        {...listeners}
        className="absolute top-2 left-2 cursor-move p-1 rounded-md hover:bg-gray-100"
        style={{
          position: 'absolute',
          top: '8px',
          left: '8px',
          cursor: 'move',
          padding: '4px',
          borderRadius: '4px',
        }}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M5 9h14M5 15h14" strokeLinecap="round" />
        </svg>
      </div>
      <div className="pl-8">{children}</div>
    </div>
  );
};

// Main ReorderableList component
const ReorderableList = ({ items, onReorder, renderItem }) => {
  const [activeId, setActiveId] = useState(null);
  
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleDragStart = (event) => {
    setActiveId(event.active.id);
  };

  const handleDragEnd = (event) => {
    const { active, over } = event;
    
    if (active.id !== over.id) {
      const oldIndex = items.findIndex(item => item.id === active.id);
      const newIndex = items.findIndex(item => item.id === over.id);
      
      onReorder(arrayMove(items, oldIndex, newIndex));
    }
    
    setActiveId(null);
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
    >
      <SortableContext
        items={items.map(item => item.id)}
        strategy={verticalListSortingStrategy}
      >
        <div className="w-full">
          {items.map(item => (
            <SortableItem key={item.id} id={item.id}>
              {renderItem(item)}
            </SortableItem>
          ))}
        </div>
      </SortableContext>
    </DndContext>
  );
};

export default ReorderableList;