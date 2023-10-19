<script>
    import RangeQuestionWidget from './RangeQuestionWidget.svelte';

    export let data;

    let feedback_form = data.feedback_form;
</script>
<h1>Feedback</h1>

<form method="POST">
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
            <textarea name={question.uuid} />
        {:else if question.question_type.type == "YesNoQuestion"}
            <input type="hidden" name={`${question.uuid}_type`} value="bool" />
            <input type="radio" name={question.uuid} value="yes" />
            <input type="radio" name={question.uuid} value="no" />
        {/if}
    {/each}

    <button type="submit">Submit</button>
</form>
