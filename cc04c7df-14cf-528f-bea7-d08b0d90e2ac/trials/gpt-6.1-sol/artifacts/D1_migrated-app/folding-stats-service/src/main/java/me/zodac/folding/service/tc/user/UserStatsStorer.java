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

import me.zodac.folding.api.tc.result.MonthlyResult;
import me.zodac.folding.service.api.FoldingStatsCore;
import me.zodac.folding.service.tc.LeaderboardStatsGenerator;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

@Service
public class UserStatsStorer {

    private static final Logger LOGGER = LogManager.getLogger();

    @Autowired
    @Lazy
    private FoldingStatsCore foldingStatsCore;

    @Autowired
    @Lazy
    private LeaderboardStatsGenerator leaderboardStatsGenerator;

    public void storeMonthlyResult() {
        final MonthlyResult monthlyResult = MonthlyResult.create(
            leaderboardStatsGenerator.generateTeamLeaderboards(),
            leaderboardStatsGenerator.generateUserCategoryLeaderboards()
        );

        if (monthlyResult.hasNoStats()) {
            LOGGER.error("Not storing result, result has no stats: {}", monthlyResult);
            return;
        }

        final MonthlyResult createdMonthlyResult = foldingStatsCore.createMonthlyResult(monthlyResult);
        LOGGER.info("Storing TC results for {}", createdMonthlyResult.getUtcTimestamp());
    }
}
