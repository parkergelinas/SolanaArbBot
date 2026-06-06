export class TradeJournal {
    events = [];
    maxEvents;
    constructor(maxEvents = 5000) {
        this.maxEvents = maxEvents;
    }
    append(event) {
        this.events.push({
            ...event,
            timestampMs: event.timestampMs ?? Date.now(),
        });
        if (this.events.length > this.maxEvents) {
            this.events.splice(0, this.events.length - this.maxEvents);
        }
    }
    all() {
        return this.events;
    }
    byType(type) {
        return this.events.filter((e) => e.type === type);
    }
    exportJson() {
        return JSON.stringify(this.events, null, 2);
    }
}
