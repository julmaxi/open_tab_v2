<script>
    import RangeQuestionWidget from './RangeQuestionWidget.svelte';

    export let data;

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

    textarea {
        width: calc(100% - 1rem);
        margin: 0.5rem;
        height: 10rem;
        border-radius: 0.25rem;
        padding: 0.5rem;
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
</style>

<div class="container">
<h1>Feedback for {data.feedback_form.target_name}</h1>
<h2>Round {data.feedback_form.target_round_index + 1}</h2>


<form method="POST">
    <div>
    {#each feedback_form.questions as question}
        <div>
            <h3>{question.full_name}</h3>

            {#if question.description.length > 0}
                <p>{question.description}</p>
            {/if}
        </div>

        {#if question.question_type.type == "RangeQuestion"}
            <input type="hidden" name={`${question.uuid}_type`} value="int" />
            <RangeQuestionWidget config={question.question_type.config} name={question.uuid} />
        {:else if question.question_type.type == "TextQuestion"}
            <input type="hidden" name={`${question.uuid}_type`} value="string" />
            <textarea name={question.uuid} placeholder="Type comments here" />
        {:else if question.question_type.type == "YesNoQuestion"}
            <div class="radio">
                <div>
                <input type="hidden" name={`${question.uuid}_type`} value="bool" />
                <div>
                <input type="radio" name={question.uuid} value="yes" />
                <label for={question.uuid}>Yes</label>
                </div>
                <div>
                <input type="radio" name={question.uuid} value="no" />
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