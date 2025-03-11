from dataclasses import dataclass, field
from typing import Optional

from selenium import webdriver
import selenium.webdriver.remote.webelement
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait, Select
from selenium.webdriver.support import expected_conditions as EC
from selenium.common.exceptions import (
    TimeoutException, 
    ElementNotInteractableException, 
    StaleElementReferenceException,
    NoSuchElementException
)
import logging
import re
import sys
import random
import time

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(sys.stdout),
        logging.FileHandler('stress_test.log')
    ]
)
logger = logging.getLogger('user_simulator')


@dataclass
class UserInfo:
    register_url: str
    submitted_ballots: set = field(default_factory=list)


HOME_URL_RE = re.compile(r".*/tournament/[a-z0-9-]+/home/[a-z0-9-]+")
LOGIN_URL_RE = re.compile(r".*/register/.*")
FEEDBACK_URL_RE = re.compile(r".*/[a-f0-9-]+/feedback/[a-f0-9-]+/\w+/\w+/debate/[a-f0-9-]+/for/[a-f0-9-]+/from/[a-f0-9-]+")
BALLOT_URL_RE = re.compile(r".*/[a-f0-9-]+/debate/([a-f0-9-]+)")
SETTINGS_URL_RE = re.compile(r".*/tournament/[a-f0-9-]+/home/[a-f0-9-]+/settings")




def wait_until_url_matches(driver: webdriver.Chrome, url_re: re.Pattern):
    wait = WebDriverWait(driver, timeout=2, poll_frequency=.2)
    wait.until(lambda _: url_re.match(driver.current_url))

    return url_re.match(driver.current_url)

def run_login_behavior(user, driver: webdriver.Chrome):
    login_button = driver.find_elements(By.ID, "link-login")
    login_button[0].click()
    wait_until_url_matches(driver, HOME_URL_RE)

@dataclass
class HomePageRoundInfo:
    role: str
    ballot_link: Optional[selenium.webdriver.remote.webelement.WebElement]
    feedback_links: list[selenium.webdriver.remote.webelement.WebElement]

@dataclass
class HomePageState:
    overdue_feedback_links: list[selenium.webdriver.remote.webelement.WebElement]
    active_round_info: list[HomePageRoundInfo]


def analyze_homepage_state(driver: webdriver.Chrome):
    overdue_div = driver.find_elements(By.XPATH, "//div[contains(@class, 'box')][//*[contains(text(), 'Overdue Feedback')]]")
    overdue_links = []
    if len(overdue_div) > 0:
        overdue_div = overdue_div[0]
        overdue_links = overdue_div.find_elements(By.TAG_NAME, "a")
    
    rounds_divs = driver.find_elements(By.XPATH, "//div[contains(@class, 'round-box')][//*[contains(text(), 'Round')]]")

    round_info = []
    for round_div in rounds_divs:
        links = round_div.find_elements(By.TAG_NAME, "a")
        ballot_link = None
        feedback_links = []
        text = round_div.text.lower()
        if "you are chair" in text:
            role = "chair"
        elif "you are wing" in text:
            role = "wing"
        elif "you are government" in text:
            role = "team"
        elif "you are opposition" in text:
            role = "team"
        elif "you are non-aligned" in text:
            role = "non-aligned"
        else:
            role = "unknown"
        for element in links:
            if re.match(r"submit\s+ballot", element.text, re.IGNORECASE):
                ballot_link = element
            elif re.match(r"submit\s+feedback", element.text, re.IGNORECASE):
                feedback_links.append(element)
        round_info.append(HomePageRoundInfo(
            role=role,
            ballot_link=ballot_link,
            feedback_links=feedback_links
        ))
            

    return HomePageState(
        overdue_feedback_links=overdue_links,
        active_round_info=round_info
    )


def click_element(driver: webdriver.Chrome, element: selenium.webdriver.remote.webelement.WebElement):
    driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", element)
    time.sleep(0.5)
    element.click()

def run_default_participant_behavior(user, driver: webdriver.Chrome):
    if not HOME_URL_RE.match(driver.current_url):
        return
    home_page_state = analyze_homepage_state(driver)
    if len(home_page_state.overdue_feedback_links) > 0:
        click_element(driver, random.choice(home_page_state.overdue_feedback_links))
        wait_until_url_matches(driver, FEEDBACK_URL_RE)

        handle_tournament_feedback(driver)
    
    ballots_to_submit = []

    for round_info in home_page_state.active_round_info:
        if round_info.role == "chair" and round_info.ballot_link:
            href = round_info.ballot_link.get_attribute("href")
            match = BALLOT_URL_RE.match(href)
            if not match.group(1) in user.submitted_ballots:
                ballots_to_submit.append(round_info)
    
    if len(ballots_to_submit) > 0:
        round_info = random.choice(ballots_to_submit)
        click_element(driver, round_info.ballot_link)
        wait_until_url_matches(driver, BALLOT_URL_RE)
        handle_ballot(driver)
        user.submitted_ballots.append(BALLOT_URL_RE.match(driver.current_url).group(1))

    if len(home_page_state.active_round_info) > 0:
        round_info = random.choice(home_page_state.active_round_info)                
        if len(round_info.feedback_links) > 0:
            click_element(driver, random.choice(round_info.feedback_links))
            wait_until_url_matches(driver, FEEDBACK_URL_RE)
            return handle_tournament_feedback(driver)

    navbar = driver.find_element(By.TAG_NAME, "nav")
    click_random_link_in_element(driver, navbar)
    if SETTINGS_URL_RE.match(driver.current_url):
        handle_settings(driver)
        
    
def click_random_link_in_element(driver, element, wait=False):
    links = element.find_elements(By.TAG_NAME, "a")
    if len(links) > 0:
        link = random.choice(links)
        logger.info(f"Randomly visiting link: {link.get_attribute('href')}")
        click_element(driver, link)
        if wait:
            href = link.get_attribute("href")
            wait_until_url_matches(driver, re.compile(re.escape(href)))
        return True
            
    return False

def select_random_radio_buttons(driver: webdriver.Chrome):
    """
    Select one random radio button from each group.
    
    Args:
        driver: Chrome WebDriver instance
    """
    radio_groups = {}
    radio_buttons = driver.find_elements(By.CSS_SELECTOR, "input[type='radio']")
    
    # Group radio buttons by name attribute
    for radio in radio_buttons:
        name = radio.get_attribute("name")
        if name:
            if name not in radio_groups:
                radio_groups[name] = []
            radio_groups[name].append(radio)
    
    # Select one random radio button from each group
    for group_name, radios in radio_groups.items():
        visible_radios = [r for r in radios if r.is_displayed() and r.is_enabled()]
        if visible_radios:
            selected_radio = random.choice(visible_radios)
            try:
                driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", selected_radio)
                time.sleep(0.5)  # Small wait for scroll
                selected_radio.click()
                logger.info(f"Selected radio button from group {group_name}")
            except Exception as e:
                logger.warning(f"Could not select radio button: {e}")

# Tournament feedback form handler
def handle_tournament_feedback(driver: webdriver.Chrome) -> bool:
    """
    Custom handler for tournament feedback forms.
    
    Args:
        driver: Chrome WebDriver instance
    
    Returns:
        True if the form was handled, False otherwise
    """
    try:
        logger.info("Processing tournament feedback form")
        
        # Find all text areas and text inputs
        text_elements = driver.find_elements(By.CSS_SELECTOR, "textarea, input[type='text']")
        
        # Fill each text element with random text
        for element in text_elements:
            if element.is_displayed() and element.is_enabled():
                try:
                    random_text = generate_random_text(50, 500)
                    driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", element)
                    time.sleep(0.5)  # Small wait for scroll
                    element.clear()
                    element.send_keys(random_text)
                    logger.info(f"Filled text element with {len(random_text)} characters")
                except Exception as e:
                    logger.warning(f"Could not fill text element: {e}")
        
        # Find all number inputs
        number_inputs = driver.find_elements(By.CSS_SELECTOR, "input[type='number']")
        
        # Fill each number input with a random number within its min/max range
        for number_input in number_inputs:
            if number_input.is_displayed() and number_input.is_enabled():
                try:
                    min_value = number_input.get_attribute("min")
                    max_value = number_input.get_attribute("max")
                    
                    # Set defaults if min/max not specified
                    min_value = int(min_value) if min_value and min_value.isdigit() else 1
                    max_value = int(max_value) if max_value and max_value.isdigit() else 100
                    
                    random_number = random.randint(min_value, max_value)
                    driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", number_input)
                    time.sleep(0.5)  # Small wait for scroll
                    number_input.clear()
                    number_input.send_keys(str(random_number))
                    logger.info(f"Filled number input with {random_number}")
                except Exception as e:
                    logger.warning(f"Could not fill number input: {e}")
        
        # Select random radio buttons
        select_random_radio_buttons(driver)
        
        # Look for a submit button
        submit_buttons = driver.find_elements(By.CSS_SELECTOR, 
                                             "button[type='submit'], input[type='submit']")
        
        submit_buttons.sort(key=lambda x: x.text.lower().strip() == "submit", reverse=True)
        
        if not submit_buttons:
            # Try to find buttons with submit-related text
            all_buttons = driver.find_elements(By.TAG_NAME, "button")
            for button in all_buttons:
                text = button.text.lower()
                if "submit" in text or "save" in text or "next" in text or "continue" in text:
                    submit_buttons.append(button)
        
        # Click the submit button if found
        if submit_buttons:
            submit_button = submit_buttons[0]
            try:
                driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", submit_button)
                time.sleep(1)  # Wait a bit longer before submitting
                submit_button.click()
                logger.info("Submitted the feedback form")
            except Exception as e:
                logger.warning(f"Could not click submit button: {e}")
        
        return True
        
    except Exception as e:
        logger.error(f"Error handling tournament feedback form: {e}")
        return False

def handle_ballot(driver: webdriver.Chrome) -> bool:
    """
    Custom handler for debate ballots.
    
    Args:
        driver: Chrome WebDriver instance
    
    Returns:
        True if the ballot was handled, False otherwise
    """
    try:
        logger.info("Processing debate ballot")
        
        # Find all number inputs
        number_inputs = driver.find_elements(By.CSS_SELECTOR, "input[type='number']")
        
        # Fill each number input with a random number within its min/max range
        if len(number_inputs) > 0:
            curr_value = number_inputs[0].get_attribute("value")
            if len(curr_value.strip()) > 0:
                print(curr_value)
                logger.info("Ballot filled. Abort.")
                return True


        for number_input in number_inputs:
            if number_input.is_displayed() and number_input.is_enabled():
                try:
                    min_value = number_input.get_attribute("min")
                    max_value = number_input.get_attribute("max")
                    
                    # Set defaults if min/max not specified
                    min_value = int(min_value) if min_value and min_value.isdigit() else 1
                    max_value = int(max_value) if max_value and max_value.isdigit() else 100
                    
                    random_number = random.randint(min_value, max_value)
                    driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", number_input)
                    time.sleep(0.5)
                    number_input.clear()
                    number_input.send_keys(str(random_number))
                    logger.info(f"Filled number input with {random_number}")
                except Exception as e:
                    logger.warning(f"Could not fill number input: {e}")

        # Track selected option values to avoid duplicates
        used_option_values = set()

        # Find all select elements
        select_elements = driver.find_elements(By.TAG_NAME, "select")
        logger.info(f"Found {len(select_elements)} select elements")
        
        
        # Process each select element
        for select_element in select_elements:
            if not select_element.is_displayed() or not select_element.is_enabled():
                continue
                
            try:
                # Create a Select object
                select = Select(select_element)
                
                # Get all options excluding disabled ones
                options = []
                for option in select.options:
                    value = option.get_attribute("value")
                    if value and not option.get_attribute("disabled") and value not in used_option_values:
                        options.append((option, value))
                
                if options:
                    # Choose a random option
                    selected_option, value = random.choice(options)
                    
                    # Scroll the select into view
                    driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", select_element)
                    time.sleep(0.5)
                    
                    # Select the option
                    select.select_by_value(value)
                    
                    # Mark this value as used
                    used_option_values.add(value)
                    
                    logger.info(f"Selected option '{selected_option.text}' with value '{value}'")
            except Exception as e:
                logger.warning(f"Could not process select element: {e}")
                        
        # Look for a submit/save button
        submit_buttons = driver.find_elements(By.CSS_SELECTOR, 
                                             "button[type='submit'], input[type='submit']")
        

        submit_buttons.sort(key=lambda x: x.text.lower().strip() == "submit", reverse=True)
        if not submit_buttons:
            # Try to find buttons with submit-related text
            all_buttons = driver.find_elements(By.TAG_NAME, "button")
            for button in all_buttons:
                text = button.text.lower()
                if "submit" in text or "save" in text or "next" in text or "continue" in text:
                    submit_buttons.append(button)
        
        # Click the submit button if found
        if submit_buttons:
            submit_button = submit_buttons[0]
            try:
                driver.execute_script("arguments[0].scrollIntoView({block: 'center'});", submit_button)
                time.sleep(1)  # Wait a bit longer before submitting
                submit_button.click()
                logger.info("Submitted the ballot")
                return True
            except Exception as e:
                logger.warning(f"Could not click submit button: {e}")
        
        return True
        
    except Exception as e:
        logger.error(f"Error handling ballot: {e}")
        return False

def handle_settings(driver: webdriver.Chrome) -> bool:
    select_random_radio_buttons(driver)
    submit_buttons = driver.find_elements(By.CSS_SELECTOR, 
                                            "button[type='submit'], input[type='submit']")
    
    submit_buttons.sort(key=lambda x: x.text.lower().strip() == "submit", reverse=True)
    
    if not submit_buttons:
        # Try to find buttons with submit-related text
        all_buttons = driver.find_elements(By.TAG_NAME, "button")
        for button in all_buttons:
            text = button.text.lower()
            if "submit" in text or "save" in text or "next" in text or "continue" in text:
                submit_buttons.append(button)

    # Click the submit button if found
    if len(submit_buttons) > 0:
        click_element(driver, submit_buttons[0])
        time.sleep(1.0)
        return True
    return False



def simulate_user_step(user, driver: webdriver.chrome.webdriver.WebDriver):
    current_url = driver.current_url

    if LOGIN_URL_RE.match(current_url):
        run_login_behavior(user, driver)
        wait_until_url_matches(driver, HOME_URL_RE)
        return
    
    if not HOME_URL_RE.match(current_url):
        driver.get(user.register_url)
        wait_until_url_matches(driver, re.compile(f"{HOME_URL_RE.pattern}|{LOGIN_URL_RE.pattern}"))
        return
    else:
        return run_default_participant_behavior(user, driver)




# Helper function to generate random text for feedback forms
def generate_random_text(min_length=50, max_length=500):
    """
    Generate random text of specified length for form fields.
    
    Args:
        min_length: Minimum length of text
        max_length: Maximum length of text
        
    Returns:
        A string of random text
    """
    length = random.randint(min_length, max_length)
    
    # Generate sentences with somewhat realistic structure
    sentences = []
    remaining_chars = length
    
    sentence_starters = [
        "The feedback is ", "I think ", "Overall ", "In my opinion ", 
        "The performance was ", "They did ", "The presentation was ",
        "This debate was ", "The arguments were ", "I found that ", 
        "It appears that ", "Considering the points made ", "When analyzing the debate ",
        "From my perspective ", "Looking at the evidence presented ",
    ]
    
    adjectives = [
        "good", "great", "excellent", "poor", "adequate", "impressive", "disappointing",
        "outstanding", "mediocre", "average", "exceptional", "subpar", "reasonable",
        "compelling", "unconvincing", "persuasive", "weak", "strong", "nuanced", "simplistic"
    ]
    
    adverbs = [
        "very", "quite", "extremely", "somewhat", "rather", "fairly", "surprisingly",
        "remarkably", "notably", "unusually", "particularly", "especially", "generally"
    ]
    
    while remaining_chars > 0:
        # Create a random sentence
        starter = random.choice(sentence_starters)
        adverb = random.choice(adverbs) if random.random() > 0.5 else ""
        adj = random.choice(adjectives)
        
        if adverb:
            sentence = f"{starter}{adverb} {adj}. "
        else:
            sentence = f"{starter}{adj}. "
        
        # Add more complex sentences occasionally
        if random.random() > 0.7 and remaining_chars > 50:
            extra = f"However, there could be improvement in some areas. "
            sentence += extra
        
        if len(sentence) <= remaining_chars:
            sentences.append(sentence)
            remaining_chars -= len(sentence)
        else:
            # If this sentence would be too long, create a shorter one
            short_sentence = f"It was {random.choice(adjectives)}. "
            if len(short_sentence) <= remaining_chars:
                sentences.append(short_sentence)
            remaining_chars = 0
    
    return "".join(sentences)

def run_user_test(register_url):
    options = Options()
    options.add_argument("--headless=new")
    driver = webdriver.Chrome(options=options)
    user = UserInfo(register_url=register_url)
    driver.get(user.register_url)
    try:
        WebDriverWait(driver, 2.0).until(
            EC.presence_of_element_located((By.TAG_NAME, "body"))
    )
    except TimeoutException:
        logger.warning("Timeout waiting for page to load")

    driver.implicitly_wait(2.0)

    for _ in range(100):
        try:
            simulate_user_step(
                user,
                driver
            )
        except Exception as e:
            pass

if __name__ == "__main__":
    run_user_test("http://localhost:5173/register/lI3MbAf6QcmpB8nygb6g5y5H8FrfeKNz1YLJbyB776MhhPuKE0mr1QCN5dODlB3d")