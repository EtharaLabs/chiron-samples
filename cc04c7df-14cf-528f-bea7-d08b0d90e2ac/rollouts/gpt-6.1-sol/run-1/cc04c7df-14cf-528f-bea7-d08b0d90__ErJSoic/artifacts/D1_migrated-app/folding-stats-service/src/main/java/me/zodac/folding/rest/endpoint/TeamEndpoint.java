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
import org.springframework.web.servlet.support.ServletUriComponentsBuilder;
import org.springframework.web.util.UriComponentsBuilder;
import javax.servlet.http.HttpServletRequest;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.context.annotation.Lazy;
import me.zodac.folding.rest.RequiredRoles;

import static me.zodac.folding.api.util.DateTimeUtils.untilNextMonthUtc;
import static me.zodac.folding.rest.response.Responses.badRequest;
import static me.zodac.folding.rest.response.Responses.cachedOk;
import static me.zodac.folding.rest.response.Responses.created;
import static me.zodac.folding.rest.response.Responses.notFound;
import static me.zodac.folding.rest.response.Responses.nullRequest;
import static me.zodac.folding.rest.response.Responses.ok;
import static me.zodac.folding.rest.response.Responses.serverError;

import java.time.temporal.ChronoUnit;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Optional;
import me.zodac.folding.api.util.StringUtils;
import me.zodac.folding.state.SystemStateManager;
import me.zodac.folding.api.state.ReadRequired;
import me.zodac.folding.api.state.SystemState;
import me.zodac.folding.api.state.WriteRequired;
import me.zodac.folding.api.tc.Team;
import me.zodac.folding.service.api.FoldingStatsCore;
import me.zodac.folding.rest.api.tc.request.TeamRequest;
import me.zodac.folding.rest.endpoint.util.IdResult;
import me.zodac.folding.rest.endpoint.util.IntegerParser;
import me.zodac.folding.rest.endpoint.util.ValidationFailureResponseMapper;
import me.zodac.folding.rest.response.BatchCreateResponse;
import me.zodac.folding.api.tc.validation.TeamValidator;
import me.zodac.folding.api.tc.validation.ValidationFailure;
import me.zodac.folding.api.tc.validation.ValidationResult;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

@RestController
@RequestMapping("/teams")
public class TeamEndpoint {

    private static final Logger LOGGER = LogManager.getLogger();

    @Autowired
    @Lazy
    private FoldingStatsCore foldingStatsCore;

    @PostMapping(value = "", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> create(@RequestBody(required = false) final TeamRequest teamRequest) {

        final ValidationResult<Team> validationResult = validateCreate(teamRequest);
        if (validationResult.isFailure()) {
            return ValidationFailureResponseMapper.map(validationResult);
        }
        final Team validatedTeam = validationResult.getOutput();

        try {
            final Team elementWithId = foldingStatsCore.createTeam(validatedTeam);

            final UriComponentsBuilder elementLocationBuilder = ServletUriComponentsBuilder.fromCurrentRequest()
                .pathSegment(String.valueOf(elementWithId.getId()));
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Created team with ID {}", elementWithId.getId());
            return created(elementWithId, elementLocationBuilder);
        } catch (final Exception e) {
            LOGGER.error("Unexpected error creating team: {}", teamRequest, e);
            return serverError();
        }
    }
    @PostMapping(value = "/batch", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> createBatchOf(@RequestBody(required = false) final Collection<TeamRequest> teamRequests) {

        final Collection<Team> validTeams = new ArrayList<>(teamRequests.size() / 2);
        final Collection<ValidationFailure> failedValidationResponses = new ArrayList<>(teamRequests.size() / 2);

        for (final TeamRequest teamRequest : teamRequests) {
            final ValidationResult<Team> validationResult = validateCreate(teamRequest);

            if (validationResult.isFailure()) {
                LOGGER.error("Found validation error for {}: {}", teamRequest, validationResult);
                failedValidationResponses.add(validationResult.getValidationFailure());
            } else {
                validTeams.add(validationResult.getOutput());
            }
        }

        if (validTeams.isEmpty()) {
            LOGGER.error("All teams contain validation errors: {}", failedValidationResponses);
            return badRequest(failedValidationResponses);
        }

        final List<Object> successful = new ArrayList<>();
        final List<Object> unsuccessful = new ArrayList<>(failedValidationResponses);

        for (final Team validTeam : validTeams) {
            try {
                final Team teamWithId = foldingStatsCore.createTeam(validTeam);
                successful.add(teamWithId);
            } catch (final Exception e) {
                LOGGER.error("Unexpected error creating team: {}", validTeam, e);
                unsuccessful.add(validTeam);
            }
        }

        final BatchCreateResponse batchCreateResponse = BatchCreateResponse.create(successful, unsuccessful);

        if (successful.isEmpty()) {
            return badRequest(batchCreateResponse);
        }

        if (!unsuccessful.isEmpty()) {
            LOGGER.error("{} teams successfully created, {} teams unsuccessful", successful.size(), unsuccessful.size());
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            return ok(batchCreateResponse);
        }

        LOGGER.info("{} teams successfully created", successful.size());
        SystemStateManager.next(SystemState.WRITE_EXECUTED);
        return ok(batchCreateResponse.getSuccessful());
    }
    @GetMapping(value = "", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getAll(final HttpServletRequest request) {

        try {
            final Collection<Team> elements = foldingStatsCore.getAllTeams();
            return cachedOk(elements, request, untilNextMonthUtc(ChronoUnit.SECONDS));
        } catch (final Exception e) {
            LOGGER.error("Unexpected error getting all teams", e);
            return serverError();
        }
    }
    @GetMapping(value = "/{teamId}", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getById(@PathVariable("teamId") final String teamId, final HttpServletRequest request) {

        try {
            final IdResult idResult = IntegerParser.parsePositive(teamId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<Team> optionalElement = foldingStatsCore.getTeam(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No team found with ID {}", teamId);
                return notFound();
            }

            final Team element = optionalElement.get();
            return cachedOk(element, request, untilNextMonthUtc(ChronoUnit.SECONDS));
        } catch (final Exception e) {
            LOGGER.error("Unexpected error getting team with ID: {}", teamId, e);
            return serverError();
        }
    }
    @GetMapping(value = "/fields", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getByHardwareName(@RequestParam(required = false, value = "teamName") final String teamName, final HttpServletRequest request) {

        try {
            if (StringUtils.isBlank(teamName)) {
                final String errorMessage = String.format("Input 'teamName' must not be blank: '%s'", teamName);
                LOGGER.error(errorMessage);
                return badRequest(errorMessage);
            }

            final Optional<Team> optionalTeam = foldingStatsCore.getAllTeams()
                .stream()
                .filter(team -> team.getTeamName().equalsIgnoreCase(teamName))
                .findAny();

            if (optionalTeam.isEmpty()) {
                LOGGER.error("No hardware found with 'hardwareName' '{}'", teamName);
                return notFound();
            }

            final Team element = optionalTeam.get();
            return cachedOk(element, request, untilNextMonthUtc(ChronoUnit.SECONDS));
        } catch (final Exception e) {
            LOGGER.error("Unexpected error getting team with 'hardwareName': '{}'", teamName, e);
            return serverError();
        }
    }
    @PutMapping(value = "/{teamId}", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> updateById(@PathVariable("teamId") final String teamId, @RequestBody(required = false) final TeamRequest teamRequest) {

        if (teamRequest == null) {
            LOGGER.error("No payload provided");
            return nullRequest();
        }

        try {
            final IdResult idResult = IntegerParser.parsePositive(teamId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<Team> optionalElement = foldingStatsCore.getTeam(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No team found with ID {}", teamId);
                return notFound();
            }
            final Team existingTeam = optionalElement.get();

            if (existingTeam.isEqualRequest(teamRequest)) {
                LOGGER.debug("No change necessary");
                return ok(existingTeam);
            }

            final ValidationResult<Team> validationResult = validateUpdate(teamRequest, existingTeam);
            if (validationResult.isFailure()) {
                return ValidationFailureResponseMapper.map(validationResult);
            }
            final Team validatedHardware = validationResult.getOutput();

            // The payload 'should' have the ID, but it's not guaranteed if the correct URL is used
            final Team teamWithId = Team.updateWithId(existingTeam.getId(), validatedHardware);
            final Team updatedTeamWithId = foldingStatsCore.updateTeam(teamWithId);

            final UriComponentsBuilder elementLocationBuilder = ServletUriComponentsBuilder.fromCurrentRequest()
                .pathSegment(String.valueOf(updatedTeamWithId.getId()));
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Updated team with ID {}", updatedTeamWithId.getId());
            return ok(updatedTeamWithId, elementLocationBuilder);
        } catch (final Exception e) {
            LOGGER.error("Unexpected error updating team with ID: {}", teamId, e);
            return serverError();
        }
    }
    @DeleteMapping(value = "/{teamId}", produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> deleteById(@PathVariable("teamId") final String teamId) {

        try {
            final IdResult idResult = IntegerParser.parsePositive(teamId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<Team> optionalElement = foldingStatsCore.getTeam(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No team found with ID {}", teamId);
                return notFound();
            }
            final Team team = optionalElement.get();

            final ValidationResult<Team> validationResult = validateDelete(team);
            if (validationResult.isFailure()) {
                return ValidationFailureResponseMapper.map(validationResult);
            }
            final Team validatedTeam = validationResult.getOutput();

            foldingStatsCore.deleteTeam(validatedTeam);
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Deleted team with ID {}", teamId);
            return ok();
        } catch (final Exception e) {
            LOGGER.error("Unexpected error deleting team with ID: {}", teamId, e);
            return serverError();
        }
    }

    private ValidationResult<Team> validateCreate(final TeamRequest teamRequest) {
        return TeamValidator.validateCreate(teamRequest, foldingStatsCore.getAllTeams());
    }

    private ValidationResult<Team> validateUpdate(final TeamRequest teamRequest, final Team existingTeam) {
        return TeamValidator.validateUpdate(teamRequest, existingTeam, foldingStatsCore.getAllTeams());
    }

    private ValidationResult<Team> validateDelete(final Team team) {
        return TeamValidator.validateDelete(team, foldingStatsCore.getAllUsersWithoutPasskeys());
    }
}
