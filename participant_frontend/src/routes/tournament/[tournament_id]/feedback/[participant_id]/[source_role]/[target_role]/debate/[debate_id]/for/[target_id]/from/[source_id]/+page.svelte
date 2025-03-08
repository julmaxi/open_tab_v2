<script>
    import RangeQuestionWidget from './RangeQuestionWidget.svelte';
    import TextFieldWidget from './TextFieldWidget.svelte';

    export let data;
    export let form;

    let feedback_form = data.feedback_form;
</script>

<style>
    button {
        margin-top: 1rem;
        margin-bottom: 1rem;
        padding: 0.5rem;
        border-radius: 0.25rem;
        background-color: rgb(34 197 94);
        color: white;
    }

    form {
        display: flex;
        flex-direction: column;
        align-items: center;
    }

    h1 {
        font-size: 1.25rem;
        font-weight: bold;
    }

    h2 {
        font-weight: lighter;
    }

    h3 {
        font-weight: bold;
    }

    .container {
        padding: 0.5rem;
    }

    .radio {
        display: flex;
        flex-direction: column;;
        align-items: center;
    }

    .validation-error {
        color: red;
    }
</style>

<div class="container">
<h1>Feedback for {data.feedback_form.target_name}</h1>
<h2>Round {data.feedback_form.target_round_index + 1}</h2>

{#if form?.validation_errors}
    <p class="validation-error">There are errors in your feedback submission.</p>
{/if}

<form method="POST">
    <div>
    {#each feedback_form.questions as question}
        <div>
            <h3>{question.full_name}</h3>

            {#if question.description.length > 0}
                <p>{question.description}</p>
            {/if}
        </div>

        {#if form?.validation_errors[question.uuid]}
            <p class="validation-error">{form.validation_errors[question.uuid]}</p>
        {/if}

        {#if question.question_type.type == "RangeQuestion"}
            <input type="hidden" name={`${question.uuid}_type`} value="int" />
            <RangeQuestionWidget config={question.question_type.config} name={question.uuid} initialValue={
                form?.values[question.uuid] || null
            } />
        {:else if question.question_type.type == "TextQuestion"}
            <input type="hidden" name={`${question.uuid}_type`} value="string" />
            <TextFieldWidget
                name={question.uuid}
                placeholder="Type comments here"
                maxLength={question.question_type.config.max_length}
                initialValue={
                    form?.values[question.uuid] || ""
                }
            />
        {:else if question.question_type.type == "YesNoQuestion"}
            <div class="radio">
                <div>
                <input type="hidden" name={`${question.uuid}_type`} value="bool" />
                <div>
                <input type="radio" name={question.uuid} value="yes" checked={form?.values[question.uuid] == true} />
                <label for={question.uuid}>Yes</label>
                </div>
                <div>
                <input type="radio" name={question.uuid} value="no" checked={form?.values[question.uuid] == false} />
                <label for={question.uuid}>No</label>
                </div>
                </div>
            </div>
        {/if}
    {/each}
    </div>

    <button type="submit">Submit</button>
</form>
</div>