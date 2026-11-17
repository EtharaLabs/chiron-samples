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
import me.zodac.folding.state.SystemStateManager;
import me.zodac.folding.api.state.ReadRequired;
import me.zodac.folding.api.state.SystemState;
import me.zodac.folding.api.state.WriteRequired;
import me.zodac.folding.api.tc.User;
import me.zodac.folding.service.api.FoldingStatsCore;
import me.zodac.folding.rest.api.tc.request.UserRequest;
import me.zodac.folding.rest.endpoint.util.IdResult;
import me.zodac.folding.rest.endpoint.util.IntegerParser;
import me.zodac.folding.rest.endpoint.util.ValidationFailureResponseMapper;
import me.zodac.folding.rest.response.BatchCreateResponse;
import me.zodac.folding.api.tc.validation.UserValidator;
import me.zodac.folding.api.tc.validation.ValidationFailure;
import me.zodac.folding.api.tc.validation.ValidationResult;
import me.zodac.folding.stats.HttpFoldingStatsRetriever;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

@RestController
@RequestMapping("/users")
public class UserEndpoint {

    private static final Logger LOGGER = LogManager.getLogger();

    @Autowired
    @Lazy
    private FoldingStatsCore foldingStatsCore;

    @PostMapping(value = "", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> create(@RequestBody(required = false) final UserRequest userRequest) {

        final ValidationResult<User> validationResult = validateCreate(userRequest);
        if (validationResult.isFailure()) {
            return ValidationFailureResponseMapper.map(validationResult);
        }
        final User validatedUser = validationResult.getOutput();

        try {
            final User elementWithId = foldingStatsCore.createUser(validatedUser);

            final UriComponentsBuilder elementLocationBuilder = ServletUriComponentsBuilder.fromCurrentRequest()
                .pathSegment(String.valueOf(elementWithId.getId()));
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Created user with ID {}", elementWithId.getId());
            return created(elementWithId, elementLocationBuilder);
        } catch (final Exception e) {
            LOGGER.error("Unexpected error creating user: {}", userRequest, e);
            return serverError();
        }
    }
    @PostMapping(value = "/batch", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> createBatchOf(@RequestBody(required = false) final Collection<UserRequest> userRequests) {

        final Collection<User> validUsers = new ArrayList<>(userRequests.size() / 2);
        final Collection<ValidationFailure> failedValidationResponses = new ArrayList<>(userRequests.size() / 2);

        for (final UserRequest userRequest : userRequests) {
            final ValidationResult<User> validationResult = validateCreate(userRequest);

            if (validationResult.isFailure()) {
                LOGGER.error("Found validation error for {}: {}", userRequest, validationResult);
                failedValidationResponses.add(validationResult.getValidationFailure());
            } else {
                validUsers.add(validationResult.getOutput());
            }
        }

        if (validUsers.isEmpty()) {
            LOGGER.error("All users contain validation errors: {}", failedValidationResponses);
            return badRequest(failedValidationResponses);
        }

        final List<Object> successful = new ArrayList<>();
        final List<Object> unsuccessful = new ArrayList<>(failedValidationResponses);

        for (final User validUser : validUsers) {
            try {
                final User userWithId = foldingStatsCore.createUser(validUser);
                successful.add(userWithId);
            } catch (final Exception e) {
                LOGGER.error("Unexpected error creating user: {}", validUser, e);
                unsuccessful.add(validUser);
            }
        }

        final BatchCreateResponse batchCreateResponse = BatchCreateResponse.create(successful, unsuccessful);

        if (successful.isEmpty()) {
            return badRequest(batchCreateResponse);
        }

        if (!unsuccessful.isEmpty()) {
            LOGGER.error("{} users successfully created, {} users unsuccessful", successful.size(), unsuccessful.size());
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            return ok(batchCreateResponse);
        }

        LOGGER.info("{} users successfully created", successful.size());
        SystemStateManager.next(SystemState.WRITE_EXECUTED);
        return ok(batchCreateResponse.getSuccessful());
    }
    @GetMapping(value = "", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getAll(final HttpServletRequest request) {

        try {
            final Collection<User> elements = foldingStatsCore.getAllUsersWithoutPasskeys();
            return cachedOk(elements, request, untilNextMonthUtc(ChronoUnit.SECONDS));
        } catch (final Exception e) {
            LOGGER.error("Unexpected error getting all users", e);
            return serverError();
        }
    }
    @GetMapping(value = "/{userId}", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getById(@PathVariable("userId") final String userId, final HttpServletRequest request) {

        try {
            final IdResult idResult = IntegerParser.parsePositive(userId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<User> optionalElement = foldingStatsCore.getUserWithoutPasskey(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No user found with ID {}", userId);
                return notFound();
            }

            final User element = optionalElement.get();
            return cachedOk(element, request, untilNextMonthUtc(ChronoUnit.SECONDS));
        } catch (final Exception e) {
            LOGGER.error("Unexpected error getting user with ID: {}", userId, e);
            return serverError();
        }
    }
    @PutMapping(value = "/{userId}", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> updateById(@PathVariable("userId") final String userId, @RequestBody(required = false) final UserRequest userRequest) {

        if (userRequest == null) {
            LOGGER.error("No payload provided");
            return nullRequest();
        }

        try {
            final IdResult idResult = IntegerParser.parsePositive(userId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<User> optionalElement = foldingStatsCore.getUserWithPasskey(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No user found with ID {}", userId);
                return notFound();
            }
            final User existingUser = optionalElement.get();

            if (existingUser.isEqualRequest(userRequest)) {
                LOGGER.debug("No change necessary");
                final User userWithHiddenPasskey = User.hidePasskey(existingUser);
                return ok(userWithHiddenPasskey);
            }

            final ValidationResult<User> validationResult = validateUpdate(userRequest, existingUser);
            if (validationResult.isFailure()) {
                return ValidationFailureResponseMapper.map(validationResult);
            }
            final User validatedUser = validationResult.getOutput();

            // The payload 'should' have the ID, but it's not guaranteed if the correct URL is used
            final User userWithId = User.updateWithId(existingUser.getId(), validatedUser);
            final User updatedUserWithId = foldingStatsCore.updateUser(userWithId, existingUser);

            final UriComponentsBuilder elementLocationBuilder = ServletUriComponentsBuilder.fromCurrentRequest()
                .pathSegment(String.valueOf(updatedUserWithId.getId()));
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Updated user with ID {}", updatedUserWithId.getId());
            return ok(updatedUserWithId, elementLocationBuilder);
        } catch (final Exception e) {
            LOGGER.error("Unexpected error updating user with ID: {}", userId, e);
            return serverError();
        }
    }
    @DeleteMapping(value = "/{userId}", produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> deleteById(@PathVariable("userId") final String userId) {

        try {
            final IdResult idResult = IntegerParser.parsePositive(userId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<User> optionalElement = foldingStatsCore.getUserWithoutPasskey(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No user found with ID {}", userId);
                return notFound();
            }
            final User user = optionalElement.get();

            final ValidationResult<User> validationResult = validateDelete(user);
            if (validationResult.isFailure()) {
                return ValidationFailureResponseMapper.map(validationResult);
            }
            final User validatedUser = validationResult.getOutput();

            foldingStatsCore.deleteUser(validatedUser);
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Deleted user with ID {}", userId);
            return ok();
        } catch (final Exception e) {
            LOGGER.error("Unexpected error deleting user with ID: {}", userId, e);
            return serverError();
        }
    }
    @GetMapping(value = "/{userId}/passkey", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> getByIdWithPasskey(@PathVariable("userId") final String userId, final HttpServletRequest request) {

        try {
            final IdResult idResult = IntegerParser.parsePositive(userId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<User> optionalElement = foldingStatsCore.getUserWithPasskey(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No user with passkey found with ID {}", userId);
                return notFound();
            }

            final User element = optionalElement.get();
            return cachedOk(element, request, untilNextMonthUtc(ChronoUnit.SECONDS));
        } catch (final Exception e) {
            LOGGER.error("Unexpected error getting user with passkey ID: {}", userId, e);
            return serverError();
        }
    }

    private ValidationResult<User> validateCreate(final UserRequest userRequest) {
        final UserValidator userValidator = UserValidator.create(HttpFoldingStatsRetriever.create());
        return userValidator.validateCreate(
            userRequest,
            foldingStatsCore.getAllUsersWithPasskeys(),
            foldingStatsCore.getAllHardware(),
            foldingStatsCore.getAllTeams()
        );
    }

    private ValidationResult<User> validateUpdate(final UserRequest userRequest, final User existingUser) {
        final UserValidator userValidator = UserValidator.create(HttpFoldingStatsRetriever.create());
        return userValidator.validateUpdate(
            userRequest,
            existingUser,
            foldingStatsCore.getAllUsersWithPasskeys(),
            foldingStatsCore.getAllHardware(),
            foldingStatsCore.getAllTeams()
        );
    }

    private ValidationResult<User> validateDelete(final User existingUser) {
        final UserValidator userValidator = UserValidator.create(HttpFoldingStatsRetriever.create());
        return userValidator.validateDelete(existingUser);
    }
}
