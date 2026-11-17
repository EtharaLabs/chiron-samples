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

package me.zodac.folding.service.tc.user;

import org.springframework.stereotype.Service;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.context.annotation.Lazy;
import org.springframework.scheduling.annotation.Async;

import static me.zodac.folding.api.util.NumberUtils.formatWithCommas;

import me.zodac.folding.api.exception.ExternalConnectionException;
import me.zodac.folding.api.stats.FoldingStatsRetriever;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.api.tc.stats.OffsetTcStats;
import me.zodac.folding.api.tc.stats.Stats;
import me.zodac.folding.api.tc.stats.UserStats;
import me.zodac.folding.api.tc.stats.UserTcStats;
import me.zodac.folding.api.util.StringUtils;
import me.zodac.folding.service.api.FoldingStatsCore;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

@Service
public class UserStatsParser {

    private static final Logger LOGGER = LogManager.getLogger();
    @Autowired
    private FoldingStatsRetriever foldingStatsRetriever;

    @Autowired
    @Lazy
    private FoldingStatsCore foldingStatsCore;

    public void parseTcStatsForUserAndWait(final User user) {
        parseTcStatsForUser(user);
    }

    @Async
    public void parseTcStatsForUser(final User user) {
        updateTcStatsForUser(user);
    }

    private void updateTcStatsForUser(final User user) {
        LOGGER.debug("Updating stats for '{}': {}", user.getDisplayName(), user);
        if (StringUtils.isBlank(user.getPasskey())) {
            LOGGER.warn("Not parsing TC stats for user, missing passkey: {}", user);
            return;
        }

        final Stats initialStats = foldingStatsCore.getInitialStats(user);
        if (initialStats.isEmpty()) {
            LOGGER.warn("Retrieved empty initial stats for user: {}", user);
            return;
        }

        final OffsetTcStats offsetTcStats = foldingStatsCore.getOffsetStats(user);
        if (offsetTcStats.isEmpty()) {
            LOGGER.trace("Retrieved empty stats offset for user: {}", () -> user);
        } else {
            LOGGER.debug("{}: {} offset points | {} offset units", user::getDisplayName,
                () -> formatWithCommas(offsetTcStats.getMultipliedPointsOffset()), () -> formatWithCommas(offsetTcStats.getUnitsOffset()));
        }

        final UserStats totalStats = getTotalStatsForUserOrEmpty(user);
        if (totalStats.isEmpty()) {
            LOGGER.warn("Retrieved empty total stats for user: {}", user);
            return;
        }

        final UserStats createdTotalStats = foldingStatsCore.createTotalStats(totalStats);
        calculateAndPersistTcStats(user, initialStats, offsetTcStats, createdTotalStats);
    }

    private UserStats getTotalStatsForUserOrEmpty(final User user) {
        try {
            return foldingStatsRetriever.getTotalStats(user);
        } catch (final ExternalConnectionException e) {
            LOGGER.warn("Error connecting to Folding@Home API at '{}'", e.getUrl(), e);
        }

        return UserStats.empty();
    }

    private void calculateAndPersistTcStats(final User user,
                                            final Stats initialStats,
                                            final OffsetTcStats offsetTcStats,
                                            final UserStats totalStats) {
        final double hardwareMultiplier = user.getHardware().getMultiplier();
        final long points = Math.max(0, totalStats.getPoints() - initialStats.getPoints());
        final long multipliedPoints = Math.round(points * hardwareMultiplier);
        final int units = Math.max(0, totalStats.getUnits() - initialStats.getUnits());

        final UserTcStats statsBeforeOffset = UserTcStats.create(user.getId(), totalStats.getTimestamp(), points, multipliedPoints, units);
        final UserTcStats hourlyUserTcStats = statsBeforeOffset.updateWithOffsets(offsetTcStats);

        LOGGER.debug("{} (ID: {}): {} total points (unmultiplied) | {} total units", user::getDisplayName, user::getId,
            () -> formatWithCommas(totalStats.getPoints()), () -> formatWithCommas(totalStats.getUnits()));
        LOGGER.debug("{} (ID: {}): {} TC multiplied points (pre-offset) | {} TC units (pre-offset)", user::getDisplayName, user::getId,
            () -> formatWithCommas(multipliedPoints), () -> formatWithCommas(units));

        final UserTcStats createdHourlyTcStats = foldingStatsCore.createHourlyTcStats(hourlyUserTcStats);
        LOGGER.info("{} (ID: {}): {} TC points | {} TC units", user.getDisplayName(), user.getId(),
            formatWithCommas(createdHourlyTcStats.getMultipliedPoints()), formatWithCommas(createdHourlyTcStats.getUnits()));
    }
}
