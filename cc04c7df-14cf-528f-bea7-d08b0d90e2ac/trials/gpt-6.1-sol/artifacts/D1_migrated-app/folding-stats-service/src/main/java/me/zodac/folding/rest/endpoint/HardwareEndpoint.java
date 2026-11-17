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
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.service.api.FoldingStatsCore;
import me.zodac.folding.rest.api.tc.request.HardwareRequest;
import me.zodac.folding.rest.endpoint.util.IdResult;
import me.zodac.folding.rest.endpoint.util.IntegerParser;
import me.zodac.folding.rest.endpoint.util.ValidationFailureResponseMapper;
import me.zodac.folding.rest.response.BatchCreateResponse;
import me.zodac.folding.api.tc.validation.HardwareValidator;
import me.zodac.folding.api.tc.validation.ValidationFailure;
import me.zodac.folding.api.tc.validation.ValidationResult;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

@RestController
@RequestMapping("/hardware")
public class HardwareEndpoint {

    private static final Logger LOGGER = LogManager.getLogger();

    @Autowired
    @Lazy
    private FoldingStatsCore foldingStatsCore;

    @PostMapping(value = "", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> create(@RequestBody(required = false) final HardwareRequest hardwareRequest) {

        final ValidationResult<Hardware> validationResult = validateCreate(hardwareRequest);
        if (validationResult.isFailure()) {
            return ValidationFailureResponseMapper.map(validationResult);
        }
        final Hardware validatedHardware = validationResult.getOutput();

        try {
            final Hardware elementWithId = foldingStatsCore.createHardware(validatedHardware);

            final UriComponentsBuilder elementLocationBuilder = ServletUriComponentsBuilder.fromCurrentRequest()
                .pathSegment(String.valueOf(elementWithId.getId()));
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Created hardware with ID {}", elementWithId.getId());
            return created(elementWithId, elementLocationBuilder);
        } catch (final Exception e) {
            LOGGER.error("Unexpected error creating hardware: {}", hardwareRequest, e);
            return serverError();
        }
    }
    @PostMapping(value = "/batch", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> createBatchOf(@RequestBody(required = false) final Collection<HardwareRequest> hardwareRequests) {

        final Collection<Hardware> validHardwares = new ArrayList<>(hardwareRequests.size() / 2);
        final Collection<ValidationFailure> failedValidationResponses = new ArrayList<>(hardwareRequests.size() / 2);

        for (final HardwareRequest hardwareRequest : hardwareRequests) {
            final ValidationResult<Hardware> validationResult = validateCreate(hardwareRequest);

            if (validationResult.isFailure()) {
                LOGGER.error("Found validation error for {}: {}", hardwareRequest, validationResult);
                failedValidationResponses.add(validationResult.getValidationFailure());
            } else {
                validHardwares.add(validationResult.getOutput());
            }
        }

        if (validHardwares.isEmpty()) {
            LOGGER.error("All hardwares contain validation errors: {}", failedValidationResponses);
            return badRequest(failedValidationResponses);
        }

        final List<Object> successful = new ArrayList<>();
        final List<Object> unsuccessful = new ArrayList<>(failedValidationResponses);

        for (final Hardware validHardware : validHardwares) {
            try {
                final Hardware hardwareWithId = foldingStatsCore.createHardware(validHardware);
                successful.add(hardwareWithId);
            } catch (final Exception e) {
                LOGGER.error("Unexpected error creating hardware: {}", validHardware, e);
                unsuccessful.add(validHardware);
            }
        }

        final BatchCreateResponse batchCreateResponse = BatchCreateResponse.create(successful, unsuccessful);

        if (successful.isEmpty()) {
            return badRequest(batchCreateResponse);
        }

        if (!unsuccessful.isEmpty()) {
            LOGGER.error("{} hardwares successfully created, {} hardwares unsuccessful", successful.size(), unsuccessful.size());
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            return ok(batchCreateResponse);
        }

        LOGGER.info("{} hardwares successfully created", successful.size());
        SystemStateManager.next(SystemState.WRITE_EXECUTED);
        return ok(batchCreateResponse.getSuccessful());
    }
    @GetMapping(value = "", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getAll(final HttpServletRequest request) {

        try {
            final Collection<Hardware> elements = foldingStatsCore.getAllHardware();
            return cachedOk(elements, request, untilNextMonthUtc(ChronoUnit.SECONDS));
        } catch (final Exception e) {
            LOGGER.error("Unexpected error getting all hardwares", e);
            return serverError();
        }
    }
    @GetMapping(value = "/{hardwareId}", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getById(@PathVariable("hardwareId") final String hardwareId, final HttpServletRequest request) {

        try {
            final IdResult idResult = IntegerParser.parsePositive(hardwareId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<Hardware> optionalElement = foldingStatsCore.getHardware(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No hardware found with ID {}", hardwareId);
                return notFound();
            }

            final Hardware element = optionalElement.get();
            return cachedOk(element, request, untilNextMonthUtc(ChronoUnit.SECONDS));
        } catch (final Exception e) {
            LOGGER.error("Unexpected error getting hardware with ID: {}", hardwareId, e);
            return serverError();
        }
    }
    @GetMapping(value = "/fields", produces = MediaType.APPLICATION_JSON_VALUE)
    @ReadRequired
    public ResponseEntity<String> getByHardwareName(@RequestParam(required = false, value = "hardwareName") final String hardwareName, final HttpServletRequest request) {

        try {
            if (StringUtils.isBlank(hardwareName)) {
                final String errorMessage = String.format("Input 'hardwareName' must not be blank: '%s'", hardwareName);
                LOGGER.error(errorMessage);
                return badRequest(errorMessage);
            }

            final Optional<Hardware> optionalHardware = foldingStatsCore.getAllHardware()
                .stream()
                .filter(hardware -> hardware.getHardwareName().equalsIgnoreCase(hardwareName))
                .findAny();

            if (optionalHardware.isEmpty()) {
                LOGGER.error("No hardware found with 'hardwareName' '{}'", hardwareName);
                return notFound();
            }

            final Hardware element = optionalHardware.get();
            return cachedOk(element, request, untilNextMonthUtc(ChronoUnit.SECONDS));
        } catch (final Exception e) {
            LOGGER.error("Unexpected error getting hardware with 'hardwareName': '{}'", hardwareName, e);
            return serverError();
        }
    }
    @PutMapping(value = "/{hardwareId}", consumes = MediaType.APPLICATION_JSON_VALUE, produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> updateById(@PathVariable("hardwareId") final String hardwareId, @RequestBody(required = false) final HardwareRequest hardwareRequest) {

        if (hardwareRequest == null) {
            LOGGER.error("No payload provided");
            return nullRequest();
        }

        try {
            final IdResult idResult = IntegerParser.parsePositive(hardwareId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<Hardware> optionalElement = foldingStatsCore.getHardware(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No hardware found with ID {}", hardwareId);
                return notFound();
            }
            final Hardware existingHardware = optionalElement.get();

            if (existingHardware.isEqualRequest(hardwareRequest)) {
                LOGGER.debug("No change necessary");
                return ok(existingHardware);
            }

            final ValidationResult<Hardware> validationResult = validateUpdate(hardwareRequest, existingHardware);
            if (validationResult.isFailure()) {
                return ValidationFailureResponseMapper.map(validationResult);
            }
            final Hardware validatedHardware = validationResult.getOutput();

            // The payload 'should' have the ID, but it's not guaranteed if the correct URL is used
            final Hardware hardwareWithId = Hardware.updateWithId(existingHardware.getId(), validatedHardware);
            final Hardware updatedHardwareWithId = foldingStatsCore.updateHardware(hardwareWithId, existingHardware);

            final UriComponentsBuilder elementLocationBuilder = ServletUriComponentsBuilder.fromCurrentRequest()
                .pathSegment(String.valueOf(updatedHardwareWithId.getId()));
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Updated hardware with ID {}", updatedHardwareWithId.getId());
            return ok(updatedHardwareWithId, elementLocationBuilder);
        } catch (final Exception e) {
            LOGGER.error("Unexpected error updating hardware with ID: {}", hardwareId, e);
            return serverError();
        }
    }
    @DeleteMapping(value = "/{hardwareId}", produces = MediaType.APPLICATION_JSON_VALUE)
    @WriteRequired
    @RequiredRoles("admin")
    public ResponseEntity<String> deleteById(@PathVariable("hardwareId") final String hardwareId) {

        try {
            final IdResult idResult = IntegerParser.parsePositive(hardwareId);
            if (idResult.isFailure()) {
                return idResult.getFailureResponse();
            }
            final int parsedId = idResult.getId();

            final Optional<Hardware> optionalElement = foldingStatsCore.getHardware(parsedId);
            if (optionalElement.isEmpty()) {
                LOGGER.error("No hardware found with ID {}", hardwareId);
                return notFound();
            }
            final Hardware hardware = optionalElement.get();

            final ValidationResult<Hardware> validationResult = validateDelete(hardware);
            if (validationResult.isFailure()) {
                return ValidationFailureResponseMapper.map(validationResult);
            }
            final Hardware validatedHardware = validationResult.getOutput();

            foldingStatsCore.deleteHardware(validatedHardware);
            SystemStateManager.next(SystemState.WRITE_EXECUTED);
            LOGGER.info("Deleted hardware with ID {}", hardwareId);
            return ok();
        } catch (final Exception e) {
            LOGGER.error("Unexpected error deleting hardware with ID: {}", hardwareId, e);
            return serverError();
        }
    }

    private ValidationResult<Hardware> validateCreate(final HardwareRequest hardwareRequest) {
        return HardwareValidator.validateCreate(hardwareRequest, foldingStatsCore.getAllHardware());
    }

    private ValidationResult<Hardware> validateUpdate(final HardwareRequest hardwareRequest, final Hardware existingHardware) {
        return HardwareValidator.validateUpdate(hardwareRequest, existingHardware, foldingStatsCore.getAllHardware());
    }

    private ValidationResult<Hardware> validateDelete(final Hardware hardware) {
        return HardwareValidator.validateDelete(hardware, foldingStatsCore.getAllUsersWithoutPasskeys());
    }
}
