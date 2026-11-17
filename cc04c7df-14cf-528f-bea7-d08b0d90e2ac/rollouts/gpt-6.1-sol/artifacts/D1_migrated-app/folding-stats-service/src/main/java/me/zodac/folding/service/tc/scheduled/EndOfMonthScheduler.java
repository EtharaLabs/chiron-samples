/*
 * MIT License
 *
 * Copyright (c) 2021-2022 zodac.me
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

package me.zodac.folding.service.tc.scheduled;

import org.springframework.stereotype.Service;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.context.annotation.Lazy;

import java.util.Calendar;
import me.zodac.folding.state.ParsingStateManager;
import me.zodac.folding.state.SystemStateManager;
import me.zodac.folding.api.state.ParsingState;
import me.zodac.folding.api.state.SystemState;
import me.zodac.folding.service.tc.lars.LarsHardwareUpdater;
import me.zodac.folding.service.tc.user.UserStatsResetter;
import me.zodac.folding.service.tc.user.UserStatsStorer;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

@Service
public class EndOfMonthScheduler {

    private static final Logger LOGGER = LogManager.getLogger();
    @org.springframework.beans.factory.annotation.Value("${ENABLE_STATS_MONTHLY_RESET:false}")
    private boolean monthlyResetEnabled;
    @org.springframework.beans.factory.annotation.Value("${ENABLE_MONTHLY_RESULT_STORAGE:false}")
    private boolean monthlyResultEnabled;
    @org.springframework.beans.factory.annotation.Value("${ENABLE_LARS_HARDWARE_UPDATE:false}")
    private boolean larsUpdateEnabled;

    @Autowired
    @Lazy
    private LarsHardwareUpdater larsHardwareUpdater;

    @Autowired
    @Lazy
    private UserStatsResetter userStatsResetter;

    @Autowired
    @Lazy
    private UserStatsStorer userStatsStorer;

    @org.springframework.scheduling.annotation.Scheduled(cron = "#{@schedules.end}", zone = "UTC")
    public void endOfTeamCompetition() {
        try {

            // Because we cannot set up a cron schedule with last day, we use the range '28-31'.
            // We then check if the current day in the month is the last day of the month.
            // If not, we skip the reset.
            if (!isLastDayInMonth()) {
                LOGGER.warn("End of month reset triggered, but not actually end of the month, skipping");
                return;
            }

            if (monthlyResultEnabled) {
                LOGGER.info("Storing TC stats for new month");
                userStatsStorer.storeMonthlyResult();
            }

            if (monthlyResetEnabled) {
                LOGGER.warn("Resetting TC stats for end of month");
                ParsingStateManager.next(ParsingState.DISABLED);
                SystemStateManager.next(SystemState.RESETTING_STATS);
                userStatsResetter.resetTeamCompetitionStats();
                SystemStateManager.next(SystemState.WRITE_EXECUTED);
            }

            if (larsUpdateEnabled) {
                LOGGER.info("Updating hardware from LARS DB");
                larsHardwareUpdater.retrieveHardwareAndPersist();
            }
        } catch (final Exception e) {
            LOGGER.error("Error with end of team schedule", e);
        }
    }

    private static boolean isLastDayInMonth() {
        final Calendar calendar = Calendar.getInstance();
        return calendar.get(Calendar.DATE) == calendar.getActualMaximum(Calendar.DATE);
    }
}
