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

package me.zodac.folding.rest.endpoint;

import org.springframework.web.bind.annotation.*;
import org.springframework.http.ResponseEntity;
import org.springframework.http.MediaType;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.context.annotation.Lazy;
import me.zodac.folding.rest.RequiredRoles;

import static java.util.stream.Collectors.toList;
import static me.zodac.folding.rest.response.Responses.notFound;
import static me.zodac.folding.rest.response.Responses.nullRequest;
import static me.zodac.folding.rest.response.Responses.ok;
import static me.zodac.folding.rest.response.Responses.serverError;

import java.util.Collection;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import me.zodac.folding.state.SystemStateManager;
import me.zodac.folding.api.state.ReadRequired;
import me.zodac.folding.api.state.SystemState;
import me.zodac.folding.api.state.WriteRequired;
import me.zodac.folding.api.tc.Category;
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.api.tc.stats.OffsetTcStats;
import me.zodac.folding.api.util.ProcessingType;
import me.zodac.folding.service.api.FoldingStatsCore;
import me.zodac.folding.service.tc.LeaderboardStatsGenerator;
import me.zodac.folding.service.tc.scheduled.StatsScheduler;
import me.zodac.folding.service.tc.user.UserStatsParser;
import me.zodac.folding.service.tc.user.UserStatsResetter;
import me.zodac.folding.rest.api.tc.CompetitionSummary;
import me.zodac.folding.rest.api.tc.UserSummary;
import me.zodac.folding.rest.api.tc.leaderboard.TeamLeaderboardEntry;
import me.zodac.folding.rest.api.tc.leaderboard.UserCategoryLeaderboardEntry;
import me.zodac.folding.rest.endpoint.util.IdResult;
import me.zodac.folding.rest.endpoint.util.IntegerParser;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

@RestController
@RequestMapping("/stats")
public class TeamCompetitionStatsEndpoint {

    private static final Logger LOGGER = LogManager.getLogger();

    @Autowired
    @Lazy
    private FoldingStatsCore foldingStatsCore;

    @Autowired
    @Lazy
    private LeaderboardStatsGenerator leaderboardStatsGenerator;

    @Autowired
    @Lazy
    private StatsScheduler statsScheduler;

    @Autowired
    @Lazy
    private UserStatsParser userStatsParser;

    @Autowired
    @Lazy
    private UserStatsResetter userStatsResetter;

    @GetMapping(value = "", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getTeamCompetitionStats() {
        LOGGER.debug("GET request received to show TC stats");

        try {
            final CompetitionSummary competitionSummary = foldingStatsCore.getCompetitionSummary();
            return ok(competitionSummary);
        } catch (final Exception e) {
            LOGGER.error("Unexpected error retrieving full TC stats", e);
            return serverError();
        }
    }
    @GetMapping(value = "/users/{userId}", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getTeamCompetitionStatsForUser(@PathVariable("userId") final String userId) {

        try {
            final IdResult idResult = IntegerParser.parsePositive(userId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<User> optionalUser = foldingStatsCore.getUserWithoutPasskey(parsedId);
            if (optionalUser.isEmpty()) {
                LOGGER.error("No user found with ID: {}", parsedId);
                return notFound();
            }
            final User user = optionalUser.get();

            final CompetitionSummary competitionSummary = foldingStatsCore.getCompetitionSummary();
            final Collection<UserSummary> userSummaries = competitionSummary.getTeams()
                .stream()
                .flatMap(teamResult -> teamResult.getActiveUsers().stream())
                .collect(toList());

            for (final UserSummary userSummary : userSummaries) {
                if (userSummary.getUser().getId() == user.getId()) {
                    return ok(userSummary);
                }
            }

            return notFound();
        } catch (final Exception e) {
            LOGGER.error("Unexpected error retrieving TC stats for users", e);
            return serverError();
        }
    }
    @PatchMapping(value = "/users/{userId}", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> updateUserWithOffset(@PathVariable("userId") final String userId, @RequestBody(required = false) final OffsetTcStats offsetTcStats) {

        if (offsetTcStats == null || offsetTcStats.isEmpty()) {
            LOGGER.error("Payload is null");
            return nullRequest();
        }

        try {
            final IdResult idResult = IntegerParser.parsePositive(userId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<User> optionalUser = foldingStatsCore.getUserWithPasskey(parsedId);
            if (optionalUser.isEmpty()) {
                LOGGER.error("No user found with ID: {}", parsedId);
                return notFound();
            }

            final User user = optionalUser.get();
            final Hardware hardware = user.getHardware();
            final OffsetTcStats offsetTcStatsToPersist = OffsetTcStats.updateWithHardwareMultiplier(offsetTcStats, hardware.getMultiplier());
            final OffsetTcStats createdOffsetStats = foldingStatsCore.createOrUpdateOffsetStats(user, offsetTcStatsToPersist);

            SystemStateManager.next(SystemState.UPDATING_STATS);
            userStatsParser.parseTcStatsForUserAndWait(user);
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Updated user with ID {} with points offset: {}", userId, createdOffsetStats);
            return ok();
        } catch (final Exception e) {
            LOGGER.error("Unexpected error updating user with ID: {}", userId, e);
            return serverError();
        }
    }
    @GetMapping(value = "/leaderboard", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getTeamLeaderboard() {
        LOGGER.debug("GET request received to show TC leaderboard");

        try {
            final List<TeamLeaderboardEntry> teamSummaries = leaderboardStatsGenerator.generateTeamLeaderboards();
            return ok(teamSummaries);
        } catch (final Exception e) {
            LOGGER.error("Unexpected error retrieving TC leaderboard", e);
            return serverError();
        }
    }
    @GetMapping(value = "/category", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getCategoryLeaderboard() {
        LOGGER.debug("GET request received to show TC category leaderboard");

        try {
            final Map<Category, List<UserCategoryLeaderboardEntry>> categoryLeaderboard =
                leaderboardStatsGenerator.generateUserCategoryLeaderboards();
            return ok(categoryLeaderboard);
        } catch (final Exception e) {
            LOGGER.error("Unexpected error retrieving TC stats", e);
            return serverError();
        }
    }
    @PostMapping(value = "/manual/update")
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> updateStats(@RequestParam(required = false, value = "async") final boolean async) {
        LOGGER.info("GET request received to manually update TC stats");

        try {
            final ProcessingType processingType = async ? ProcessingType.ASYNCHRONOUS : ProcessingType.SYNCHRONOUS;
            statsScheduler.manualTeamCompetitionStatsParsing(processingType);
            return ok();
        } catch (final Exception e) {
            LOGGER.error("Unexpected error manually parsing TC stats", e);
            return serverError();
        }
    }
    @PostMapping(value = "/manual/reset")
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> resetStats() {
        LOGGER.info("GET request received to manually reset TC stats");

        try {
            SystemStateManager.next(SystemState.RESETTING_STATS);
            userStatsResetter.resetTeamCompetitionStats();
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            return ok();
        } catch (final Exception e) {
            LOGGER.error("Unexpected error manually resetting TC stats", e);
            return serverError();
        }
    }
}
