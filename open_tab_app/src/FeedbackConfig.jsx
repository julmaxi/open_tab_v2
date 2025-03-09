import { useContext, useEffect, useState } from "react";
import { executeAction } from "./Action";
import Button from "./UI/Button";
import { open } from "@tauri-apps/api/dialog";
import { useFloating, offset, flip, shift } from '@floating-ui/react';

import { TournamentContext } from "./TournamentContext";
import { ErrorHandlingContext } from "./Action";
import { useView } from "./View";

import { SplitDetailView } from "./UI/SplitDetailView";

import React from 'react';
import FeedbackQuestionEditor, { QUESTION_TYPES } from './FeedbackQuestionEditor';
import { Popover } from "./UI/Popover";
import { VisibilityConfigurator } from "./VisibilityConfigurator";
import ReorderableList from "./ReorderableList";
import { Toolbar, ToolbarButton } from "./UI/Toolbar";

function FormView({ form, questions, onUpdateQuestion, onUpdateForm, onAddQuestionToForm }) {
    console.log(questions, form);
    const [isOpen, setIsOpen] = useState(false);
    const trigger = (
        <button className="flex items-center justify-center w-full h-12 bg-gray-200 border shadow-inner hover:bg-gray-300">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-6 h-6">
                <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="1.5" fill="none" />
                <path strokeLinecap="round" strokeLinejoin="round" d="M12 8v8m4-4H8" />
            </svg>
        </button>
    );

    const formQuestions = form.questions.map((question) => {
        return { ...questions[question], uuid: question };
    });

    const addable_questions = Object.entries(questions).filter(([questionId, question]) => {
        return !form.questions.includes(questionId);
    });
    addable_questions.sort(([aId, a], [bId, b]) => {
        return a.full_name.localeCompare(b.full_name);
    });

    return (
        <div>
            <div>
                <label className="block text-sm font-medium">Name</label>
                <input
                    type="text"
                    className="w-full p-2 border rounded"
                    value={form.name}
                    onChange={(e) => {
                        let newForm = { ...form };
                        newForm.name = e.target.value;
                        onUpdateForm(newForm);
                    }}
                />
            </div>

            <VisibilityConfigurator visibility={form.visibility} onUpdateVisibility={
                (visibility) => {
                    let newForm = { ...form };
                    newForm.visibility = visibility;
                    onUpdateForm(newForm);
                }
            } />

            <div>
                <ReorderableList
                    items={form.questions.map((questionId) => ({
                        id: questionId
                    }))}
                    onReorder={(newOrder) => {
                        let newForm = { ...form };
                        newForm.questions = newOrder.map((item) => item.id);
                        onUpdateForm(newForm);
                    }}
                    renderItem={(item) => {
                        const question = questions[item.id];
                        return (
                                <FeedbackQuestionEditor
                                    question={question}
                                    onUpdate={(newQuestion) => {
                                        onUpdateQuestion(item.id, newQuestion);
                                    }}
                                    onRemove={() => {
                                        let newForm = { ...form };
                                        newForm.questions = newForm.questions.filter((q) => q !== item.id);
                                        onUpdateForm(newForm);
                                    }}
                                />
                        );
                    }}
                />
            </div>

            <Popover trigger={trigger} isOpen={isOpen} onOpen={() => setIsOpen(true)} onClose={() => setIsOpen(false)}>
                <ul>
                    {Object.entries(QUESTION_TYPES).map(([questionType, { makeNewConfig, displayName }]) => (
                        <li key={questionType} onClick={() => {
                            setIsOpen(false);
                            onAddQuestionToForm({
                                type: questionType,
                                short_name: "new_question",
                                full_name: "New Question",
                                description: "",
                                is_required: false,
                                is_confidential: false,
                                ...makeNewConfig()
                            });
                        }}>
                            New {displayName}
                        </li>
                    ))}
                    <li>
                        <SubMenu title={"Add Existing Question…"}>
                            <ul>
                                {addable_questions.map(([questionId, question]) => (
                                    <li key={questionId} onClick={() => {
                                        setIsOpen(false);
                                        onAddQuestionToForm(questionId);
                                    }}>
                                        {question.full_name} ({question.short_name})
                                    </li>
                                ))}
                            </ul>
                        </SubMenu>
                    </li>
                </ul>
            </Popover>
        </div>
    );
}

function SubMenu({ children, title }) {
    const [isOpen, setIsOpen] = useState(false);
    return (
        <Popover trigger={<button>{title}</button>} isOpen={isOpen} onOpen={() => setIsOpen(true)} onClose={() => setIsOpen(false)}>
            {children}
        </Popover>
    );
}

export function FeedbackConfigRoute() {
    const tournamentId = useContext(TournamentContext).uuid;
    const errorContext = useContext(ErrorHandlingContext);

    const feedback_forms = useView({ type: "FeedbackForms", tournament_id: tournamentId }, { forms: [], questions: {} });

    const [selectedFormIndex, setSelectedFormIndex] = useState(null);
    const [selectedForm, setSelectedForm] = useState(null);

    const [forms, setForms] = useState([]);
    const [questions, setQuestions] = useState({});

    const [nextNewId, setNextNewId] = useState(0);

    const [hasChanges, setHasChanges] = useState(false);

    useEffect(() => {
        if (selectedFormIndex !== null) {
            setSelectedForm(forms[selectedFormIndex] || null);
        }
    }, [selectedFormIndex, forms]);

    useEffect(() => {
        setForms(feedback_forms.forms);
        setQuestions(feedback_forms.questions);
    }, [feedback_forms]);

    useEffect(() => {
        //Check if at least one form is modified
        let origForms = new Set(feedback_forms.forms);
        let newForms = new Set(forms);

        let intersect = new Set([...origForms].filter(x => newForms.has(x)));
        let union = new Set([...origForms, ...newForms]);

        let hasChanges = intersect.size !== union.size;

        if (!hasChanges) {
            let origQuestions = new Set(Object.values(feedback_forms.questions));
            let newQuestions = new Set(Object.values(questions));

            intersect = new Set([...origQuestions].filter(x => newQuestions.has(x)));
            union = new Set([...origQuestions, ...newQuestions]);

            hasChanges = intersect.size !== union.size;
        }

        setHasChanges(hasChanges);
    }, [forms, questions, feedback_forms]);


    const addNewForm = () => {
        const newForm = {
            uuid: `new_form_${nextNewId}`,
            name: "new_form",
            visibility: {},
            questions: []
        };
        setForms([...forms, newForm]);
        setSelectedFormIndex(forms.length);

        setNextNewId(nextNewId + 1);
    };

    const removeForm = (formUuid) => {
        setForms(forms.filter(form => form.uuid !== formUuid));
        if (selectedForm && selectedForm.uuid === formUuid) {
            setSelectedFormIndex(null);
            setSelectedForm(null);
        }
    };

    return (
        <div className="flex align-middle justify-center flex-col h-full w-full">
            <div className="flex-1 min-h-0">
                <SplitDetailView initialDetailWidth={1000}>
                    <div className="h-full flex flex-col">
                    <ul className="flex-1 overflow-y-auto">
                        {forms.map((form, idx) => (
                            <li key={form.uuid} className="relative">
                                <span onClick={() => setSelectedFormIndex(idx)}>
                                    {form.name}
                                    <span className="text-blue-500">{
                                        !feedback_forms.forms.includes(form) ? "*" : ""
                                    }</span>
                                </span>
                                <button
                                    className="absolute top-0 right-0 text-red-500"
                                    onClick={() => removeForm(form.uuid)}
                                >
                                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4">
                                        <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                                    </svg>
                                </button>
                            </li>
                        ))}
                    </ul>
                    </div>
                    <div>
                        {selectedForm !== null ? (
                            <FormView
                                form={selectedForm}
                                questions={questions}
                                onUpdateQuestion={(questionId, question) => {
                                    setQuestions((prevQuestions) => {
                                        let newQuestions = { ...prevQuestions };
                                        newQuestions[questionId] = question;
                                        return newQuestions;
                                    });
                                }}
                                onUpdateForm={(form) => {
                                    setForms((prevForms) => {
                                        let newForms = [...prevForms];
                                        newForms[selectedFormIndex] = form;
                                        return newForms;
                                    });
                                }}
                                onAddQuestionToForm={(questionOrQuestionId) => {
                                    if (typeof questionOrQuestionId === "string") {
                                        let newForm = { ...selectedForm };
                                        newForm.questions.push(questionOrQuestionId);
                                        setForms((prevForms) => {
                                            let newForms = [...prevForms];
                                            newForms[selectedFormIndex] = newForm;
                                            return newForms;
                                        });
                                    } else {
                                        let newQuestionId = `new_question_${nextNewId}`;
                                        setNextNewId(nextNewId + 1);
                                        setQuestions((prevQuestions) => {
                                            let newQuestions = { ...prevQuestions };
                                            newQuestions[newQuestionId] = questionOrQuestionId;
                                            return newQuestions;
                                        });
                                        let newForm = { ...selectedForm };
                                        newForm.questions.push(newQuestionId);
                                        setForms((prevForms) => {
                                            let newForms = [...prevForms];
                                            newForms[selectedFormIndex] = newForm;
                                            return newForms;
                                        });
                                    }
                                }}
                            />
                        ) : (
                            <p>Select a form</p>
                        )}
                    </div>
                </SplitDetailView>
            </div>
            <Toolbar>
                <ToolbarButton icon="add" onClick={addNewForm}>Add Form</ToolbarButton>
                {
                    hasChanges ? (
                        <ToolbarButton icon="refresh" onClick={() => {
                            setSelectedForm(null);
                            setSelectedFormIndex(null);
                            setForms(feedback_forms.forms);
                            setQuestions(feedback_forms.questions);
                        }
                        }>Discard Changes</ToolbarButton>
                    ) : null
                }
                {
                    hasChanges ? (
                        <ToolbarButton icon="save" onClick={() => {
                            setSelectedForm(null);
                            setSelectedFormIndex(null);
                            executeAction("UpdateFeedbackSystem", {
                                tournament_id: tournamentId,
                                forms,
                                questions,
                            }).catch(errorContext.setError);
                        }}>Save Changes</ToolbarButton>
                    ) : null
                }
            </Toolbar>
        </div>
    );
}