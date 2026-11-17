package me.zodac.folding;

import org.springframework.core.env.Environment;
import org.springframework.stereotype.Component;

/** Each process computes its schedules from its own Spring environment. */
@Component("schedules")
public class Schedules {
    private final Environment environment;
    public Schedules(Environment environment) { this.environment = environment; }
    private String value(String key, String fallback) { return environment.getProperty(key, fallback); }
    private boolean enabled(String key) { return Boolean.parseBoolean(value(key, "false")); }
    public String getStats() {
        if (!enabled("ENABLE_STATS_SCHEDULED_PARSING")) return "-";
        return value("STATS_PARSING_SCHEDULE_SECOND", "0") + " " +
            value("STATS_PARSING_SCHEDULE_MINUTE", "55") + " " +
            value("STATS_PARSING_SCHEDULE_HOUR", "*") + " " +
            value("STATS_PARSING_SCHEDULE_FIRST_DAY_OF_MONTH", "3") + "-" +
            value("STATS_PARSING_SCHEDULE_LAST_DAY_OF_MONTH", "31") + " * *";
    }
    public String getStart() {
        return enabled("ENABLE_STATS_MONTHLY_RESET") ? "0 15 0 " + value("STATS_PARSING_SCHEDULE_FIRST_DAY_OF_MONTH", "3") + " * *" : "-";
    }
    public String getEnd() {
        return enabled("ENABLE_STATS_MONTHLY_RESET") || enabled("ENABLE_MONTHLY_RESULT_STORAGE") || enabled("ENABLE_LARS_HARDWARE_UPDATE") ? "0 57 23 28-31 * *" : "-";
    }
}
