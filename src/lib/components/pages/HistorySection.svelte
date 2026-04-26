<script lang="ts">
  export type HistorySummaryTone = "blue" | "purple" | "green" | "orange";

  export type HistorySummaryCard = {
    tone: HistorySummaryTone;
    label: string;
    value: string;
    hint: string;
  };

  export type HistoryDayRow = {
    day: string;
    chars: string;
    duration: string;
    speed: string;
    saved: string;
  };

  type Props = {
    summaryCards: HistorySummaryCard[];
    dayRows: HistoryDayRow[];
    byDayTitle: string;
    byDayDescription: string;
    dateColumnLabel: string;
    inputCharsLabel: string;
    voiceDurationLabel: string;
    averageSpeedLabel: string;
    savedTimeLabel: string;
  };

  let {
    summaryCards,
    dayRows,
    byDayTitle,
    byDayDescription,
    dateColumnLabel,
    inputCharsLabel,
    voiceDurationLabel,
    averageSpeedLabel,
    savedTimeLabel,
  }: Props = $props();
</script>

<section class="history-page">
  <section class="history-summary">
    {#each summaryCards as card}
      <article class={`history-card ${card.tone}`}>
        <p>{card.label}</p>
        <strong>{card.value}</strong>
        <span>{card.hint}</span>
      </article>
    {/each}
  </section>

  <section class="daily-panel form-panel">
    <div class="section-heading">
      <h3>{byDayTitle}</h3>
      <p>{byDayDescription}</p>
    </div>
    <div class="day-list">
      <div class="day-list-head">
        <span>{dateColumnLabel}</span>
        <span>{inputCharsLabel}</span>
        <span>{voiceDurationLabel}</span>
        <span>{averageSpeedLabel}</span>
        <span>{savedTimeLabel}</span>
      </div>
      {#each dayRows as day}
        <article>
          <span>{day.day}</span>
          <strong>{day.chars}</strong>
          <span>{day.duration}</span>
          <span>{day.speed}</span>
          <strong>{day.saved}</strong>
        </article>
      {/each}
    </div>
  </section>
</section>

<style>
  .history-page {
    display: grid;
    gap: 14px;
    max-width: 1120px;
  }

  .history-summary {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 12px;
  }

  .history-card,
  .form-panel {
    min-width: 0;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 16px;
    box-shadow: var(--shadow-card);
  }

  .history-card {
    min-height: 104px;
    padding: 16px;
  }

  .history-card p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 700;
    text-transform: none;
  }

  .history-card strong {
    display: block;
    margin-top: 10px;
    color: var(--text-main);
    font-size: 20px;
    font-weight: 800;
    line-height: 1.2;
    overflow-wrap: anywhere;
  }

  .history-card span {
    display: block;
    margin-top: 8px;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .history-card.blue {
    border-top: 4px solid var(--primary);
  }

  .history-card.purple {
    border-top: 4px solid var(--gradient-end);
  }

  .history-card.green {
    border-top: 4px solid var(--success);
  }

  .history-card.orange {
    border-top: 4px solid #f97316;
  }

  .daily-panel {
    min-width: 0;
    padding: 18px;
  }

  .section-heading {
    display: grid;
    gap: 4px;
  }

  .section-heading h3 {
    margin: 0 0 6px;
    color: var(--text-main);
    font-size: 18px;
    font-weight: 800;
  }

  .section-heading p {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .day-list {
    display: grid;
    gap: 0;
    min-width: 0;
    overflow: hidden;
  }

  .day-list-head,
  .day-list article {
    display: grid;
    grid-template-columns: minmax(120px, 1.05fr) repeat(4, minmax(92px, 1fr));
    align-items: center;
    gap: 12px;
    min-height: 48px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .day-list-head {
    min-height: 34px;
    padding-top: 0;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 700;
  }

  .day-list article:last-child {
    border-bottom: 0;
  }

  .day-list span {
    min-width: 0;
    color: var(--text-secondary);
    font-size: 14px;
    overflow-wrap: anywhere;
  }

  .day-list strong {
    min-width: 0;
    color: var(--text-main);
    font-size: 15px;
    font-weight: 800;
    overflow-wrap: anywhere;
  }

  .day-list-head span:nth-child(n + 2),
  .day-list article span:nth-child(n + 3),
  .day-list article strong {
    text-align: right;
  }

  .day-list article span:first-child {
    text-align: left;
  }

  @media (max-width: 1180px) {
    .day-list-head,
    .day-list article {
      grid-template-columns: minmax(104px, 1fr) repeat(4, minmax(78px, 0.82fr));
      gap: 8px;
    }
  }
</style>
