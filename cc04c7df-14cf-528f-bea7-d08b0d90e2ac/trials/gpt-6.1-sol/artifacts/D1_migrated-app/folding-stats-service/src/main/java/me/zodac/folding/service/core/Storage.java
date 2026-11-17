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

package me.zodac.folding.service.core;

import org.springframework.stereotype.Service;
import org.springframework.beans.factory.annotation.Autowired;

import static me.zodac.folding.cache.CompetitionSummaryCache.COMPETITION_SUMMARY_ID;

import java.time.Month;
import java.time.Year;
import java.util.Collection;
import java.util.Collections;
import java.util.Optional;
import me.zodac.folding.api.UserAuthenticationResult;
import me.zodac.folding.api.db.DbManager;
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.api.tc.Team;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.api.tc.result.MonthlyResult;
import me.zodac.folding.api.tc.stats.OffsetTcStats;
import me.zodac.folding.api.tc.stats.RetiredUserTcStats;
import me.zodac.folding.api.tc.stats.UserStats;
import me.zodac.folding.api.tc.stats.UserTcStats;
import me.zodac.folding.cache.CompetitionSummaryCache;
import me.zodac.folding.cache.HardwareCache;
import me.zodac.folding.cache.InitialStatsCache;
import me.zodac.folding.cache.OffsetTcStatsCache;
import me.zodac.folding.cache.RetiredTcStatsCache;
import me.zodac.folding.cache.TcStatsCache;
import me.zodac.folding.cache.TeamCache;
import me.zodac.folding.cache.TotalStatsCache;
import me.zodac.folding.cache.UserCache;
import me.zodac.folding.rest.api.tc.CompetitionSummary;
import me.zodac.folding.rest.api.tc.historic.HistoricStats;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

@Service
public class Storage {

    private static final Logger LOGGER = LogManager.getLogger();
    @Autowired
    private DbManager dbManager;

    // POJO caches
    private final HardwareCache hardwareCache = HardwareCache.getInstance();
    private final TeamCache teamCache = TeamCache.getInstance();
    private final UserCache userCache = UserCache.getInstance();

    // Stats caches
    private final CompetitionSummaryCache competitionSummaryCache = CompetitionSummaryCache.getInstance();
    private final InitialStatsCache initialStatsCache = InitialStatsCache.getInstance();
    private final OffsetTcStatsCache offsetTcStatsCache = OffsetTcStatsCache.getInstance();
    private final RetiredTcStatsCache retiredTcStatsCache = RetiredTcStatsCache.getInstance();
    private final TcStatsCache tcStatsCache = TcStatsCache.getInstance();
    private final TotalStatsCache totalStatsCache = TotalStatsCache.getInstance();

    @Cached(HardwareCache.class)
    public Hardware createHardware(final Hardware hardware) {
        final Hardware hardwareWithId = dbManager.createHardware(hardware);
        hardwareCache.add(hardwareWithId.getId(), hardwareWithId);
        return hardwareWithId;
    }

    @Cached(HardwareCache.class)
    public Optional<Hardware> getHardware(final int hardwareId) {
        final Optional<Hardware> fromCache = hardwareCache.get(hardwareId);

        if (fromCache.isPresent()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Get hardware");
        final Optional<Hardware> fromDb = dbManager.getHardware(hardwareId);
        fromDb.ifPresent(hardware -> hardwareCache.add(hardwareId, hardware));
        return fromDb;
    }

    @Cached(HardwareCache.class)
    public Collection<Hardware> getAllHardware() {
        final Collection<Hardware> fromCache = hardwareCache.getAll();

        if (!fromCache.isEmpty()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Get all hardware");
        final Collection<Hardware> fromDb = dbManager.getAllHardware();

        for (final Hardware hardware : fromDb) {
            hardwareCache.add(hardware.getId(), hardware);
        }

        return fromDb;
    }

    @Cached({HardwareCache.class, UserCache.class})
    public Hardware updateHardware(final Hardware hardwareToUpdate) {
        final Hardware updatedHardware = dbManager.updateHardware(hardwareToUpdate);
        hardwareCache.add(updatedHardware.getId(), updatedHardware);

        getAllUsers()
            .stream()
            .filter(user -> user.getHardware().getId() == updatedHardware.getId())
            .map(user -> User.updateHardware(user, updatedHardware))
            .forEach(updatedUser -> userCache.add(updatedUser.getId(), updatedUser));

        return updatedHardware;
    }

    @Cached(HardwareCache.class)
    public void deleteHardware(final int hardwareId) {
        dbManager.deleteHardware(hardwareId);
        hardwareCache.remove(hardwareId);
    }

    @Cached(TeamCache.class)
    public Team createTeam(final Team team) {
        final Team teamWithId = dbManager.createTeam(team);
        teamCache.add(teamWithId.getId(), teamWithId);
        return teamWithId;
    }

    @Cached(TeamCache.class)
    public Optional<Team> getTeam(final int teamId) {
        final Optional<Team> fromCache = teamCache.get(teamId);

        if (fromCache.isPresent()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Get team");
        final Optional<Team> fromDb = dbManager.getTeam(teamId);
        fromDb.ifPresent(team -> teamCache.add(teamId, team));
        return fromDb;
    }

    @Cached(TeamCache.class)
    public Collection<Team> getAllTeams() {
        final Collection<Team> fromCache = teamCache.getAll();

        if (!fromCache.isEmpty()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Get all teams");
        final Collection<Team> fromDb = dbManager.getAllTeams();

        for (final Team team : fromDb) {
            teamCache.add(team.getId(), team);
        }

        return fromDb;
    }

    @Cached({TeamCache.class, UserCache.class})
    public Team updateTeam(final Team teamToUpdate) {
        final Team updatedTeam = dbManager.updateTeam(teamToUpdate);
        teamCache.add(updatedTeam.getId(), updatedTeam);

        getAllUsers()
            .stream()
            .filter(user -> user.getTeam().getId() == updatedTeam.getId())
            .map(user -> User.updateTeam(user, updatedTeam))
            .forEach(updatedUser -> userCache.add(updatedUser.getId(), updatedUser));

        return updatedTeam;
    }

    @Cached(TeamCache.class)
    public void deleteTeam(final int teamId) {
        dbManager.deleteTeam(teamId);
        teamCache.remove(teamId);
    }

    @Cached(UserCache.class)
    public User createUser(final User user) {
        final User userWithId = dbManager.createUser(user);
        userCache.add(userWithId.getId(), userWithId);
        return userWithId;
    }

    @Cached(UserCache.class)
    public Optional<User> getUser(final int userId) {
        final Optional<User> fromCache = userCache.get(userId);

        if (fromCache.isPresent()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Get user");
        final Optional<User> fromDb = dbManager.getUser(userId);
        fromDb.ifPresent(user -> userCache.add(userId, user));
        return fromDb;
    }

    @Cached(UserCache.class)
    public Collection<User> getAllUsers() {
        final Collection<User> fromCache = userCache.getAll();

        if (!fromCache.isEmpty()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Get all users");
        final Collection<User> fromDb = dbManager.getAllUsers();

        for (final User user : fromDb) {
            userCache.add(user.getId(), user);
        }

        return fromDb;
    }

    @Cached(UserCache.class)
    public User updateUser(final User userToUpdate) {
        final User updatedUser = dbManager.updateUser(userToUpdate);
        userCache.add(updatedUser.getId(), updatedUser);
        return updatedUser;
    }

    @Cached({UserCache.class, InitialStatsCache.class, OffsetTcStatsCache.class, TcStatsCache.class, TotalStatsCache.class})
    public void deleteUser(final int userId) {
        dbManager.deleteUser(userId);
        userCache.remove(userId);

        // Remove the user entry from all stats caches
        offsetTcStatsCache.remove(userId);
        totalStatsCache.remove(userId);
        initialStatsCache.remove(userId);
        tcStatsCache.remove(userId);
    }

    @NotCached
    public MonthlyResult createMonthlyResult(final MonthlyResult monthlyResult) {
        return dbManager.createMonthlyResult(monthlyResult);
    }

    @NotCached
    public Optional<MonthlyResult> getMonthlyResult(final Month month, final Year year) {
        return dbManager.getMonthlyResult(month, year);
    }

    @Cached(RetiredTcStatsCache.class)
    public RetiredUserTcStats createRetiredUserStats(final RetiredUserTcStats retiredUserTcStats) {
        final RetiredUserTcStats createdRetiredUserTcStats = dbManager.createRetiredUserStats(retiredUserTcStats);
        retiredTcStatsCache.add(createdRetiredUserTcStats.getRetiredUserId(), createdRetiredUserTcStats);
        return createdRetiredUserTcStats;
    }

    @Cached(RetiredTcStatsCache.class)
    public Collection<RetiredUserTcStats> getAllRetiredUsers() {
        final Collection<RetiredUserTcStats> fromCache = retiredTcStatsCache.getAll();

        if (!fromCache.isEmpty()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Get all retired users");
        final Collection<RetiredUserTcStats> fromDb = dbManager.getAllRetiredUserStats();

        for (final RetiredUserTcStats retiredUserTcStats : fromDb) {
            retiredTcStatsCache.add(retiredUserTcStats.getRetiredUserId(), retiredUserTcStats);
        }

        return fromDb;
    }

    @Cached(RetiredTcStatsCache.class)
    public void deleteAllRetiredUserTcStats() {
        dbManager.deleteAllRetiredUserStats();
        retiredTcStatsCache.removeAll();
    }

    @NotCached
    public UserAuthenticationResult authenticateSystemUser(final String userName, final String password) {
        return dbManager.authenticateSystemUser(userName, password);
    }

    @NotCached
    public Collection<HistoricStats> getHistoricStats(final int userId, final Year year, final Month month, final int day) {
        if (year == null) {
            return Collections.emptyList();
        }

        if (month == null) {
            return dbManager.getHistoricStatsMonthly(userId, year);
        }

        if (day == 0) {
            return dbManager.getHistoricStatsDaily(userId, year, month);
        }

        return dbManager.getHistoricStatsHourly(userId, year, month, day);
    }

    @Cached(TotalStatsCache.class)
    public UserStats createTotalStats(final UserStats userStats) {
        final UserStats fromDb = dbManager.createTotalStats(userStats);
        totalStatsCache.add(fromDb.getUserId(), fromDb);
        return fromDb;
    }

    @Cached(TotalStatsCache.class)
    public Optional<UserStats> getTotalStats(final int userId) {
        final Optional<UserStats> fromCache = totalStatsCache.get(userId);

        if (fromCache.isPresent()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Total stats");
        final Optional<UserStats> fromDb = dbManager.getTotalStats(userId);
        fromDb.ifPresent(userStats -> totalStatsCache.add(userId, userStats));
        return fromDb;
    }

    @Cached(OffsetTcStatsCache.class)
    public OffsetTcStats createOrUpdateOffsetStats(final int userId, final OffsetTcStats offsetTcStats) {
        final OffsetTcStats fromDb = dbManager.createOrUpdateOffsetStats(userId, offsetTcStats);
        offsetTcStatsCache.add(userId, fromDb);
        return fromDb;
    }

    @Cached(OffsetTcStatsCache.class)
    public Optional<OffsetTcStats> getOffsetStats(final int userId) {
        final Optional<OffsetTcStats> fromCache = offsetTcStatsCache.get(userId);

        if (fromCache.isPresent()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Offset stats");
        final Optional<OffsetTcStats> fromDb = dbManager.getOffsetStats(userId);
        fromDb.ifPresent(offsetTcStats -> offsetTcStatsCache.add(userId, offsetTcStats));
        return fromDb;
    }

    @Cached(OffsetTcStatsCache.class)
    public void deleteOffsetStats(final int userId) {
        dbManager.deleteOffsetStats(userId);
        offsetTcStatsCache.remove(userId);
    }

    @Cached(OffsetTcStatsCache.class)
    public void deleteAllOffsetTcStats() {
        dbManager.deleteAllOffsetStats();
        offsetTcStatsCache.removeAll();
    }

    @Cached(TcStatsCache.class)
    public UserTcStats createHourlyTcStats(final UserTcStats userTcStats) {
        final UserTcStats fromDb = dbManager.createHourlyTcStats(userTcStats);
        tcStatsCache.add(userTcStats.getUserId(), fromDb);
        return fromDb;
    }

    @Cached(TcStatsCache.class)
    public Optional<UserTcStats> getHourlyTcStats(final int userId) {
        final Optional<UserTcStats> fromCache = tcStatsCache.get(userId);

        if (fromCache.isPresent()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Hourly TC stats");
        final Optional<UserTcStats> fromDb = dbManager.getHourlyTcStats(userId);
        fromDb.ifPresent(userTcStats -> tcStatsCache.add(userId, userTcStats));
        return fromDb;
    }

    @NotCached
    public Optional<UserTcStats> getFirstHourlyTcStats() {
        return dbManager.getFirstHourlyTcStats();
    }

    @Cached(InitialStatsCache.class)
    public UserStats createInitialStats(final UserStats userStats) {
        final UserStats fromDb = dbManager.createInitialStats(userStats);
        initialStatsCache.add(fromDb.getUserId(), fromDb);
        return fromDb;
    }

    @Cached(InitialStatsCache.class)
    public Optional<UserStats> getInitialStats(final int userId) {
        final Optional<UserStats> fromCache = initialStatsCache.get(userId);

        if (fromCache.isPresent()) {
            return fromCache;
        }

        LOGGER.trace("Cache miss! Initial stats");
        final Optional<UserStats> fromDb = dbManager.getInitialStats(userId);
        fromDb.ifPresent(userStats -> initialStatsCache.add(userId, userStats));
        return fromDb;
    }

    @Cached(CompetitionSummaryCache.class)
    public CompetitionSummary createCompetitionSummary(final CompetitionSummary competitionSummary) {
        this.competitionSummaryCache.add(COMPETITION_SUMMARY_ID, competitionSummary);
        return competitionSummary;
    }

    @Cached(CompetitionSummaryCache.class)
    public Optional<CompetitionSummary> getCompetitionSummary() {
        return competitionSummaryCache.get(COMPETITION_SUMMARY_ID);
    }

    @Cached(TotalStatsCache.class)
    public void evictTcStatsCache() {
        tcStatsCache.removeAll();
    }

    @Cached(InitialStatsCache.class)
    public void evictInitialStatsCache() {
        initialStatsCache.removeAll();
    }

    @Cached({
        CompetitionSummaryCache.class,
        HardwareCache.class,
        InitialStatsCache.class,
        OffsetTcStatsCache.class,
        RetiredTcStatsCache.class,
        TcStatsCache.class,
        TeamCache.class,
        TotalStatsCache.class,
        UserCache.class
    })
    public void printCacheContents() {
        // POJOs
        LOGGER.info("HardwareCache: {}", hardwareCache.getCacheContents());
        LOGGER.info("TeamCache: {}", teamCache.getCacheContents());
        LOGGER.info("UserCache: {}", userCache.getCacheContents());

        // Stats
        LOGGER.info("InitialStatsCache: {}", initialStatsCache.getCacheContents());
        LOGGER.info("OffsetStatsCache: {}", offsetTcStatsCache.getCacheContents());
        LOGGER.info("RetiredTcStatsCache: {}", retiredTcStatsCache.getCacheContents());
        LOGGER.info("TcStatsCache: {}", tcStatsCache.getCacheContents());
        LOGGER.info("TotalStatsCache: {}", totalStatsCache.getCacheContents());

        // TC overall
        LOGGER.info("CompetitionSummaryCache: {}", competitionSummaryCache.getCacheContents());
    }
}
